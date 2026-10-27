use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use ignore::{WalkBuilder, WalkState, overrides::Override, overrides::OverrideBuilder};
use serde::Serialize;

/// Generated files that bloat counts but never need refactoring.
const DEFAULT_EXCLUDES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "bun.lock",
    "bun.lockb",
    "poetry.lock",
    "uv.lock",
    "Pipfile.lock",
    "go.sum",
    "composer.lock",
    "Gemfile.lock",
    "*.min.js",
    "*.min.css",
    "*.map",
];

pub struct WalkOptions<'a> {
    pub include: &'a [String],
    pub exclude: &'a [String],
    pub hidden: bool,
    pub default_excludes: bool,
}

pub struct FileEntry {
    /// Canonical path; the identity used to count each file once.
    pub real: PathBuf,
    /// `real` relative to the canonical root.
    pub display: PathBuf,
}

/// Entries the walk saw but did not count, so nothing disappears silently.
#[derive(Debug, Default, Serialize)]
pub struct Skipped {
    /// Symlinks whose target does not exist.
    pub broken_links: usize,
    /// Symlinks resolving outside the root.
    pub outside_root: usize,
    /// Directory symlinks. Never descended: an in-root target is walked by its
    /// real path anyway, so following them could only double count or loop.
    pub dir_links: usize,
    /// Symlinks to a file already counted, or whose target is excluded.
    pub duplicates: usize,
    /// Unreadable entries (permissions, races with deletion).
    pub errors: usize,
}

impl Skipped {
    pub fn total(&self) -> usize {
        [
            self.broken_links,
            self.outside_root,
            self.dir_links,
            self.duplicates,
            self.errors,
        ]
        .iter()
        .fold(0, |a, b| a.saturating_add(*b))
    }
}

pub struct Walk {
    pub files: Vec<FileEntry>,
    pub skipped: Skipped,
}

#[derive(Default)]
struct State {
    seen: HashSet<PathBuf>,
    files: Vec<FileEntry>,
    skipped: Skipped,
}

enum Visit {
    File(PathBuf),
    Skip(fn(&mut Skipped) -> &mut usize),
    Nothing,
}

/// Collect files under `root` in parallel, honoring .gitignore/.ignore and
/// include/exclude globs. Each file is keyed by its realpath, so symlinks can
/// neither loop nor count a file twice.
pub fn collect_files(root: &Path, opts: &WalkOptions) -> Result<Walk> {
    let root =
        dunce::canonicalize(root).with_context(|| format!("cannot resolve {}", root.display()))?;
    let overrides = build_overrides(&root, opts)?;

    let state = Mutex::new(State::default());
    WalkBuilder::new(&root)
        .hidden(!opts.hidden)
        .follow_links(false)
        .overrides(overrides.clone())
        .build_parallel()
        .run(|| {
            let (root, overrides, state) = (&root, &overrides, &state);
            Box::new(move |entry| {
                let visit = match entry {
                    Ok(entry) => classify(&entry, root, overrides),
                    Err(_) => Visit::Skip(|s| &mut s.errors),
                };
                // A poisoned lock only means another visitor panicked; the data is still valid.
                let mut state = state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                match visit {
                    Visit::File(real) => state.add(real, root),
                    Visit::Skip(field) => bump(field(&mut state.skipped)),
                    Visit::Nothing => {}
                }
                WalkState::Continue
            })
        });

    let State {
        mut files, skipped, ..
    } = state
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Parallel visit order varies; sort for deterministic output.
    files.sort_by(|a, b| a.display.cmp(&b.display));
    Ok(Walk { files, skipped })
}

impl State {
    fn add(&mut self, real: PathBuf, root: &Path) {
        if !self.seen.insert(real.clone()) {
            bump(&mut self.skipped.duplicates);
            return;
        }
        let display = real.strip_prefix(root).unwrap_or(&real).to_path_buf();
        self.files.push(FileEntry { real, display });
    }
}

fn bump(n: &mut usize) {
    *n = n.saturating_add(1);
}

fn classify(entry: &ignore::DirEntry, root: &Path, overrides: &Override) -> Visit {
    let Some(file_type) = entry.file_type() else {
        return Visit::Nothing;
    };
    if file_type.is_file() {
        // The root is canonical and directory links are never followed, so a
        // regular file's walk path has no symlink components: it is its realpath.
        return Visit::File(entry.path().to_path_buf());
    }
    if !file_type.is_symlink() {
        return Visit::Nothing;
    }
    let Ok(real) = dunce::canonicalize(entry.path()) else {
        return Visit::Skip(|s| &mut s.broken_links);
    };
    if !real.starts_with(root) {
        return Visit::Skip(|s| &mut s.outside_root);
    }
    if real.is_dir() {
        return Visit::Skip(|s| &mut s.dir_links);
    }
    // The link itself passed the filters; its target must too.
    if overrides.matched(&real, false).is_ignore() {
        return Visit::Skip(|s| &mut s.duplicates);
    }
    Visit::File(real)
}

fn build_overrides(root: &Path, opts: &WalkOptions) -> Result<Override> {
    let mut overrides = OverrideBuilder::new(root);
    for glob in opts.include {
        overrides.add(glob)?;
    }
    let defaults = DEFAULT_EXCLUDES.iter().filter(|_| opts.default_excludes);
    for glob in opts
        .exclude
        .iter()
        .map(String::as_str)
        .chain(defaults.copied())
    {
        overrides.add(&format!("!{glob}"))?;
    }
    Ok(overrides.build()?)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn opts() -> WalkOptions<'static> {
        WalkOptions {
            include: &[],
            exclude: &[],
            hidden: false,
            default_excludes: true,
        }
    }

    fn names(walk: &Walk) -> Vec<String> {
        walk.files
            .iter()
            .map(|f| f.display.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn symlinks_resolve_dedupe_and_never_loop() {
        let tmp = std::env::temp_dir().join(format!("sarah-walk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let root = tmp.join("repo");
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::write(root.join("src/a.rs"), "fn a() {}\n").unwrap();
        fs::write(tmp.join("outside.rs"), "fn o() {}\n").unwrap();

        symlink(root.join("src/a.rs"), root.join("alias.rs")).unwrap(); // duplicate of a file
        symlink(&root, root.join("src/nested/loop")).unwrap(); // cycle back to the root
        symlink(root.join("src"), root.join("src_link")).unwrap(); // dir link inside root
        symlink(tmp.join("outside.rs"), root.join("out.rs")).unwrap(); // escapes root
        symlink(root.join("missing.rs"), root.join("broken.rs")).unwrap(); // dangling

        let walk = collect_files(&root, &opts()).unwrap();
        assert_eq!(names(&walk), vec!["src/a.rs"]);
        assert_eq!(walk.skipped.duplicates, 1);
        assert_eq!(walk.skipped.dir_links, 2);
        assert_eq!(walk.skipped.outside_root, 1);
        assert_eq!(walk.skipped.broken_links, 1);

        // Walking through a symlinked root reports paths relative to the real root.
        let via_link = tmp.join("repo-link");
        symlink(&root, &via_link).unwrap();
        assert_eq!(
            names(&collect_files(&via_link, &opts()).unwrap()),
            vec!["src/a.rs"]
        );

        // A link whose target is excluded does not smuggle the target back in.
        let excluded = ["src/**".to_string()];
        let walk = collect_files(
            &root,
            &WalkOptions {
                exclude: &excluded,
                ..opts()
            },
        )
        .unwrap();
        assert!(names(&walk).is_empty());
        fs::remove_dir_all(&tmp).unwrap();
    }
}
