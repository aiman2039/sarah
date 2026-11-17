use std::path::Path;

use anyhow::{Context, Result};
use globset::{GlobMatcher, GlobSet};

use crate::config::{DEFAULT_TEST_GLOBS, GlobHelpers};

/// Built-in `entry_points` besides the test globs: files that run without being imported
/// (binaries, crate roots, benches, examples, scripts, package initializers, tool configs).
const DEFAULT_ENTRY_POINTS: &[&str] = &[
    "**/main.rs",
    "**/lib.rs",
    "**/build.rs",
    "**/benches/**",
    "**/examples/**",
    "**/bin/**",
    "**/fuzz/**",
    "**/scripts/**",
    "**/__init__.py",
    "**/__main__.py",
    "**/main.py",
    "**/manage.py",
    "**/setup.py",
    "**/conftest.py",
    "**/index.*",
    "**/main.{js,jsx,ts,tsx}",
    "**/*.config.*",
    "**/*.d.ts",
];

/// Top-level graph rules from `sarah.toml` and the CLI: `layers`, `orphan_files`, `entry_points`.
#[derive(Debug, Default)]
pub struct GraphRules {
    layers: Vec<(String, GlobMatcher)>,
    /// Entry points; `None` while the orphan rule is off.
    entry_points: Option<GlobSet>,
}

/// Raw graph settings, compiled by `GraphRules::new`.
pub struct GraphRulesInput<'a> {
    pub layers: &'a [String],
    pub entry_points: &'a [String],
    pub orphans: bool,
}

impl GraphRules {
    pub fn new(input: GraphRulesInput<'_>) -> Result<Self> {
        let GraphRulesInput {
            layers,
            entry_points,
            orphans,
        } = input;
        let layers = layers
            .iter()
            .map(|glob| Ok((glob.clone(), GlobHelpers::path_matcher(glob)?)))
            .collect::<Result<_>>()
            .context("in `layers` of sarah.toml")?;
        let entry_points = orphans
            .then(|| Self::entry_set(entry_points))
            .transpose()
            .context("in `entry_points` of sarah.toml")?;
        Ok(Self {
            layers,
            entry_points,
        })
    }

    /// `true` when a rule here needs the import graph.
    pub(super) fn is_active(&self) -> bool {
        !self.layers.is_empty() || self.entry_points.is_some()
    }

    pub(super) fn has_layers(&self) -> bool {
        !self.layers.is_empty()
    }

    /// Index of the first layer glob matching `path`.
    pub(super) fn layer_of(&self, path: &Path) -> Option<usize> {
        self.layers.iter().position(|(_, m)| m.is_match(path))
    }

    pub(super) fn layer_glob(&self, layer: usize) -> &str {
        self.layers.get(layer).map_or("", |(glob, _)| glob.as_str())
    }

    pub(super) fn entry_points(&self) -> Option<&GlobSet> {
        self.entry_points.as_ref()
    }

    /// The test globs, the other built-in entry points, then `extra`.
    fn entry_set(extra: &[String]) -> Result<GlobSet> {
        let defaults = DEFAULT_TEST_GLOBS
            .iter()
            .chain(DEFAULT_ENTRY_POINTS)
            .copied();
        GlobHelpers::path_set(defaults.chain(extra.iter().map(String::as_str)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_matching_layer_wins_and_defaults_are_entry_points() {
        let rules = GraphRules::new(GraphRulesInput {
            layers: &["src/units/**".into(), "src/**".into()],
            entry_points: &["src/app.rs".into()],
            orphans: true,
        })
        .unwrap();
        assert_eq!(rules.layer_of(Path::new("src/units/a.rs")), Some(0));
        assert_eq!(rules.layer_of(Path::new("src/cli/a.rs")), Some(1));
        assert_eq!(rules.layer_of(Path::new("docs/a.md")), None);
        let entries = rules.entry_points().unwrap();
        for path in [
            "src/main.rs",
            "tests/a.rs",
            "pkg/__init__.py",
            "web/index.ts",
            "src/app.rs",
        ] {
            assert!(entries.is_match(path), "{path}");
        }
        assert!(!entries.is_match("src/a.rs"));
    }
}
