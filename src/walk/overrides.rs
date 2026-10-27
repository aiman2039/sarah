use std::path::Path;

use anyhow::Result;
use ignore::overrides::{Override, OverrideBuilder};

use super::walker::WalkFilters;

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

impl WalkFilters<'_> {
    pub(super) fn overrides(&self, root: &Path) -> Result<Override> {
        let Self {
            include,
            exclude,
            default_excludes,
        } = self;
        let mut overrides = OverrideBuilder::new(root);
        for glob in *include {
            overrides.add(glob)?;
        }
        let defaults = DEFAULT_EXCLUDES.iter().filter(|_| *default_excludes);
        for glob in exclude.iter().map(String::as_str).chain(defaults.copied()) {
            overrides.add(&format!("!{glob}"))?;
        }
        Ok(overrides.build()?)
    }
}
