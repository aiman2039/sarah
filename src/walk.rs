use std::path::{Path, PathBuf};

use anyhow::Result;
use ignore::{WalkBuilder, overrides::OverrideBuilder};

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

/// Collect files under `root`, honoring .gitignore/.ignore and include/exclude globs.
pub fn collect_files(root: &Path, opts: &WalkOptions) -> Result<Vec<PathBuf>> {
    let mut overrides = OverrideBuilder::new(root);
    for glob in opts.include {
        overrides.add(glob)?;
    }
    let defaults = DEFAULT_EXCLUDES.iter().filter(|_| opts.default_excludes);
    for glob in opts.exclude.iter().map(String::as_str).chain(defaults.copied()) {
        overrides.add(&format!("!{glob}"))?;
    }

    let mut files: Vec<PathBuf> = WalkBuilder::new(root)
        .hidden(!opts.hidden)
        .overrides(overrides.build()?)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .map(|e| e.into_path())
        .collect();
    files.sort();
    Ok(files)
}
