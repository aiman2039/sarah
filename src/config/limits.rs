use std::path::Path;

use anyhow::{Context, Result};
use globset::GlobMatcher;
use serde::Deserialize;

use super::glob::GlobHelpers;
use crate::units::{Cognitive, Cyclomatic, Depth, Lines, Params, Tokens};

/// Size limits that apply to one file and its functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub file_tokens: Tokens,
    pub file_lines: Lines,
    pub fn_lines: Lines,
    pub fn_tokens: Option<Tokens>,
    pub fn_depth: Option<Depth>,
    pub fn_params: Option<Params>,
    pub fn_cyclomatic: Option<Cyclomatic>,
    pub fn_cognitive: Option<Cognitive>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            file_tokens: Tokens(8000),
            file_lines: Lines(500),
            fn_lines: Lines(80),
            fn_tokens: None,
            fn_depth: None,
            fn_params: None,
            fn_cyclomatic: None,
            fn_cognitive: None,
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
    pub max_fn_params: Option<Params>,
    pub max_fn_cyclomatic: Option<Cyclomatic>,
    pub max_fn_cognitive: Option<Cognitive>,
}

/// One `max_*` key of a `[[files]]` rule (and of `LimitPatch`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LimitKey {
    FileTokens,
    FileLines,
    FnLines,
    FnTokens,
    FnDepth,
    FnParams,
    FnCyclomatic,
    FnCognitive,
}

impl LimitKey {
    pub const ALL: [Self; 8] = [
        Self::FileTokens,
        Self::FileLines,
        Self::FnLines,
        Self::FnTokens,
        Self::FnDepth,
        Self::FnParams,
        Self::FnCyclomatic,
        Self::FnCognitive,
    ];

    /// The `sarah.toml` key.
    pub fn name(self) -> &'static str {
        match self {
            Self::FileTokens => "max_file_tokens",
            Self::FileLines => "max_file_lines",
            Self::FnLines => "max_fn_lines",
            Self::FnTokens => "max_fn_tokens",
            Self::FnDepth => "max_fn_depth",
            Self::FnParams => "max_fn_params",
            Self::FnCyclomatic => "max_fn_cyclomatic",
            Self::FnCognitive => "max_fn_cognitive",
        }
    }
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
    pub max_fn_params: Option<Params>,
    pub max_fn_cyclomatic: Option<Cyclomatic>,
    pub max_fn_cognitive: Option<Cognitive>,
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
            max_fn_params,
            max_fn_cyclomatic,
            max_fn_cognitive,
        } = *patch;
        self.file_tokens = max_file_tokens.unwrap_or(self.file_tokens);
        self.file_lines = max_file_lines.unwrap_or(self.file_lines);
        self.fn_lines = max_fn_lines.unwrap_or(self.fn_lines);
        self.fn_tokens = max_fn_tokens.or(self.fn_tokens);
        self.fn_depth = max_fn_depth.or(self.fn_depth);
        self.fn_params = max_fn_params.or(self.fn_params);
        self.fn_cyclomatic = max_fn_cyclomatic.or(self.fn_cyclomatic);
        self.fn_cognitive = max_fn_cognitive.or(self.fn_cognitive);
    }
}

impl LimitPatch {
    pub fn get(&self, key: LimitKey) -> Option<usize> {
        match key {
            LimitKey::FileTokens => self.max_file_tokens.map(|v| v.0),
            LimitKey::FileLines => self.max_file_lines.map(|v| v.0),
            LimitKey::FnLines => self.max_fn_lines.map(|v| v.0),
            LimitKey::FnTokens => self.max_fn_tokens.map(|v| v.0),
            LimitKey::FnDepth => self.max_fn_depth.map(|v| v.0),
            LimitKey::FnParams => self.max_fn_params.map(|v| v.0),
            LimitKey::FnCyclomatic => self.max_fn_cyclomatic.map(|v| v.0),
            LimitKey::FnCognitive => self.max_fn_cognitive.map(|v| v.0),
        }
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
            max_fn_params: self.max_fn_params,
            max_fn_cyclomatic: self.max_fn_cyclomatic,
            max_fn_cognitive: self.max_fn_cognitive,
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

    /// Index of the last `[[files]]` rule that matches `path` and sets `key` (CLI flags
    /// ignored): the rule whose value applies to the path.
    pub fn governing_rule(&self, path: &Path, key: LimitKey) -> Option<usize> {
        self.rules
            .iter()
            .rposition(|(matcher, patch)| patch.get(key).is_some() && matcher.is_match(path))
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
