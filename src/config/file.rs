use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::limits::{BaseFlags, FileRule};
use crate::structure::{FolderRule, Preset};
use crate::units::Lines;

/// Looked up in the analyzed root when `--config` is not given.
const CONFIG_FILE: &str = "sarah.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Folder rules applied before the `[[folder]]` rules.
    pub preset: Option<Preset>,
    /// Report `unused_allow` warnings unless a matching `[[files]]` rule overrides it.
    pub unused_allow: Option<bool>,
    /// Reject `sarah-allow:` comments with an empty reason.
    pub require_allow_reason: Option<bool>,
    /// Strip inline unit tests (`#[cfg(test)]`, Zig `test`) before counting.
    pub exclude_inline_tests: Option<bool>,
    /// Report circular file dependencies (`a -> b -> a`).
    pub no_cycles: Option<bool>,
    /// Warn when duplicate code blocks exceed this many normalized lines.
    pub max_duplicate_lines: Option<Lines>,
    /// Added to `--include`.
    #[serde(default)]
    pub include: Vec<String>,
    /// Added to `--exclude`.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Size limits per glob; later matches override earlier ones field by field.
    #[serde(default)]
    pub files: Vec<FileRule>,
    /// Folder-structure rules; later matches override earlier ones field by field.
    #[serde(default)]
    pub folder: Vec<FolderRule>,
}

impl Config {
    /// `explicit` (from `--config`) must exist; otherwise `<root>/sarah.toml` is read when present.
    pub fn load(root: &Path, explicit: Option<&Path>) -> Result<Self> {
        let Some(path) = Self::locate(root, explicit) else {
            return Ok(Self::default());
        };
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read config {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("invalid config {}", path.display()))
    }

    pub fn base_flags(&self) -> BaseFlags {
        BaseFlags {
            unused_allow: self.unused_allow.unwrap_or(false),
            require_allow_reason: self.require_allow_reason.unwrap_or(false),
            exclude_inline_tests: self.exclude_inline_tests.unwrap_or(false),
        }
    }

    fn locate(root: &Path, explicit: Option<&Path>) -> Option<PathBuf> {
        if explicit.is_some() {
            return Some(Self::target(root, explicit));
        }
        let default = Self::target(root, None);
        default.is_file().then_some(default)
    }

    /// Where the config is (or would be): `explicit`, else `<root>/sarah.toml`.
    pub fn target(root: &Path, explicit: Option<&Path>) -> PathBuf {
        explicit.map_or_else(|| root.join(CONFIG_FILE), Path::to_path_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_config() {
        let config: Config = toml::from_str(
            r#"
            preset = "rust"
            unused_allow = true
            require_allow_reason = true
            exclude_inline_tests = true
            no_cycles = true
            max_duplicate_lines = 12
            exclude = ["vendor/**"]

            [[files]]
            path = "tests/**"
            max_fn_lines = 200

            [[folder]]
            path = "src/**"
            max_files = 12
            naming = "snake_case"
            "#,
        )
        .unwrap();
        assert_eq!(config.preset, Some(Preset::Rust));
        assert_eq!(config.unused_allow, Some(true));
        assert_eq!(config.no_cycles, Some(true));
        assert_eq!(config.max_duplicate_lines, Some(Lines(12)));
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(toml::from_str::<Config>("[[folder]]\npath = \"src\"\nmax_flies = 3\n").is_err());
    }
}
