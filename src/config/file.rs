use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::limits::FileRule;
use crate::structure::{FolderRule, Preset};

/// Looked up in the analyzed root when `--config` is not given.
const CONFIG_FILE: &str = "sarah.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Folder rules applied before the `[[folder]]` rules.
    pub preset: Option<Preset>,
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

    fn locate(root: &Path, explicit: Option<&Path>) -> Option<PathBuf> {
        if let Some(path) = explicit {
            return Some(path.to_path_buf());
        }
        let default = root.join(CONFIG_FILE);
        default.is_file().then_some(default)
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
        assert_eq!(config.exclude, vec!["vendor/**"]);
        assert_eq!(config.files.len(), 1);
        assert_eq!(config.folder.len(), 1);
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(toml::from_str::<Config>("[[folder]]\npath = \"src\"\nmax_flies = 3\n").is_err());
    }
}
