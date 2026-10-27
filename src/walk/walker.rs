use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use ignore::{WalkBuilder, WalkState, overrides::Override};

use super::classify::Visit;
use super::skipped::Skipped;

pub struct WalkOptions<'a> {
    pub filters: WalkFilters<'a>,
    pub hidden: bool,
}

/// Path filters; every field feeds the `ignore` override set.
pub struct WalkFilters<'a> {
    pub include: &'a [String],
    pub exclude: &'a [String],
    pub default_excludes: bool,
}

pub struct FileEntry {
    /// Canonical path; the identity used to count each file once.
    pub real: PathBuf,
    /// `real` relative to the canonical root.
    pub display: PathBuf,
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

impl Walk {
    /// Collect files under `root` in parallel, honoring .gitignore/.ignore and
    /// include/exclude globs. Each file is keyed by its realpath, so symlinks can
    /// neither loop nor count a file twice.
    pub fn collect(root: &Path, opts: &WalkOptions) -> Result<Self> {
        let WalkOptions { filters, hidden } = opts;
        let root = Self::canonical_root(root)?;
        let overrides = filters.overrides(&root)?;
        let state = Self::visit_in_parallel(&root, &overrides, *hidden);
        Ok(state.into_walk())
    }

    fn canonical_root(root: &Path) -> Result<PathBuf> {
        dunce::canonicalize(root).with_context(|| format!("cannot resolve {}", root.display()))
    }

    fn visit_in_parallel(root: &Path, overrides: &Override, hidden: bool) -> State {
        let state = Mutex::new(State::default());
        WalkBuilder::new(root)
            .hidden(!hidden)
            .follow_links(false)
            .overrides(overrides.clone())
            .build_parallel()
            .run(|| {
                let state = &state;
                Box::new(move |entry| {
                    let visit = Visit::of(entry, root, overrides);
                    // A poisoned lock only means another visitor panicked; the data is still valid.
                    state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .record(visit, root);
                    WalkState::Continue
                })
            });
        state
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl State {
    fn record(&mut self, visit: Visit, root: &Path) {
        match visit {
            Visit::File(real) => self.add(real, root),
            Visit::Skip(field) => {
                let count = field(&mut self.skipped);
                *count = count.saturating_add(1);
            }
            Visit::Nothing => {}
        }
    }

    fn into_walk(self) -> Walk {
        let Self {
            mut files, skipped, ..
        } = self;
        // Parallel visit order varies; sort for deterministic output.
        files.sort_by(|a, b| a.display.cmp(&b.display));
        Walk { files, skipped }
    }

    fn add(&mut self, real: PathBuf, root: &Path) {
        if !self.seen.insert(real.clone()) {
            self.skipped.duplicates = self.skipped.duplicates.saturating_add(1);
            return;
        }
        let display = real.strip_prefix(root).unwrap_or(&real).to_path_buf();
        self.files.push(FileEntry { real, display });
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn opts() -> WalkOptions<'static> {
        WalkOptions {
            filters: WalkFilters {
                include: &[],
                exclude: &[],
                default_excludes: true,
            },
            hidden: false,
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

        let walk = Walk::collect(&root, &opts()).unwrap();
        assert_eq!(names(&walk), vec!["src/a.rs"]);
        assert_eq!(walk.skipped.duplicates, 1);
        assert_eq!(walk.skipped.dir_links, 2);
        assert_eq!(walk.skipped.outside_root, 1);
        assert_eq!(walk.skipped.broken_links, 1);

        // Walking through a symlinked root reports paths relative to the real root.
        let via_link = tmp.join("repo-link");
        symlink(&root, &via_link).unwrap();
        assert_eq!(
            names(&Walk::collect(&via_link, &opts()).unwrap()),
            vec!["src/a.rs"]
        );

        // A link whose target is excluded does not smuggle the target back in.
        let excluded = ["src/**".to_string()];
        let walk = Walk::collect(
            &root,
            &WalkOptions {
                filters: WalkFilters {
                    exclude: &excluded,
                    ..opts().filters
                },
                ..opts()
            },
        )
        .unwrap();
        assert!(names(&walk).is_empty());
        fs::remove_dir_all(&tmp).unwrap();
    }
}
