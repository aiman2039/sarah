use std::path::Path;

use anyhow::{Context, Result};
use globset::GlobMatcher;
use serde::Deserialize;

use super::glob::GlobHelpers;
use crate::units::{Depth, Lines, Tokens};

/// Size limits that apply to one file and its functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub file_tokens: Tokens,
    pub file_lines: Lines,
    pub fn_lines: Lines,
    pub fn_tokens: Option<Tokens>,
    pub fn_depth: Option<Depth>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            file_tokens: Tokens(8000),
            file_lines: Lines(500),
            fn_lines: Lines(80),
            fn_tokens: None,
            fn_depth: None,
        }
    }
}

/// Limits that a rule or the CLI sets; `None` keeps the earlier value.
#[derive(Clone, Copy, Debug, Default)]
#[expect(
    clippy::struct_field_names,
    reason = "names mirror the CLI flags and sarah.toml keys"
)]
pub struct LimitPatch {
    pub max_file_tokens: Option<Tokens>,
    pub max_file_lines: Option<Lines>,
    pub max_fn_lines: Option<Lines>,
    pub max_fn_tokens: Option<Tokens>,
    pub max_fn_depth: Option<Depth>,
}

/// A `[[files]]` entry in `sarah.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRule {
    pub path: String,
    pub max_file_tokens: Option<Tokens>,
    pub max_file_lines: Option<Lines>,
    pub max_fn_lines: Option<Lines>,
    pub max_fn_tokens: Option<Tokens>,
    pub max_fn_depth: Option<Depth>,
}

/// Resolves the limits for a path: defaults, then matching `[[files]]` rules in order,
/// then CLI flags.
pub struct LimitRules {
    rules: Vec<(GlobMatcher, LimitPatch)>,
    cli: LimitPatch,
}

impl Limits {
    fn apply(&mut self, patch: &LimitPatch) {
        let LimitPatch {
            max_file_tokens,
            max_file_lines,
            max_fn_lines,
            max_fn_tokens,
            max_fn_depth,
        } = *patch;
        self.file_tokens = max_file_tokens.unwrap_or(self.file_tokens);
        self.file_lines = max_file_lines.unwrap_or(self.file_lines);
        self.fn_lines = max_fn_lines.unwrap_or(self.fn_lines);
        self.fn_tokens = max_fn_tokens.or(self.fn_tokens);
        self.fn_depth = max_fn_depth.or(self.fn_depth);
    }
}

impl FileRule {
    fn patch(&self) -> LimitPatch {
        LimitPatch {
            max_file_tokens: self.max_file_tokens,
            max_file_lines: self.max_file_lines,
            max_fn_lines: self.max_fn_lines,
            max_fn_tokens: self.max_fn_tokens,
            max_fn_depth: self.max_fn_depth,
        }
    }
}

impl LimitRules {
    pub fn new(files: &[FileRule], cli: LimitPatch) -> Result<Self> {
        let rules = files
            .iter()
            .map(|rule| {
                let matcher = GlobHelpers::path_matcher(&rule.path)
                    .context("in a [[files]] rule of sarah.toml")?;
                Ok((matcher, rule.patch()))
            })
            .collect::<Result<_>>()?;
        Ok(Self { rules, cli })
    }

    pub fn for_path(&self, path: &Path) -> Limits {
        let mut limits = Limits::default();
        for (matcher, patch) in &self.rules {
            if matcher.is_match(path) {
                limits.apply(patch);
            }
        }
        limits.apply(&self.cli);
        limits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_merge_in_order_and_cli_wins() {
        let config: crate::config::Config = toml::from_str(
            "[[files]]\npath = \"**\"\nmax_fn_lines = 60\nmax_fn_depth = 4\n\
             [[files]]\npath = \"tests/**\"\nmax_fn_lines = 200\n",
        )
        .unwrap();
        let cli = LimitPatch {
            max_file_lines: Some(Lines(300)),
            ..LimitPatch::default()
        };
        let rules = LimitRules::new(&config.files, cli).unwrap();

        let src = rules.for_path(Path::new("src/a.rs"));
        assert_eq!(src.fn_lines, Lines(60));
        assert_eq!(src.fn_depth, Some(Depth(4)));
        assert_eq!(src.file_lines, Lines(300));
        assert_eq!(src.file_tokens, Tokens(8000));

        let test = rules.for_path(Path::new("tests/cli.rs"));
        assert_eq!(test.fn_lines, Lines(200));
        assert_eq!(test.fn_depth, Some(Depth(4)));
    }
}
