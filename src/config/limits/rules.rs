use std::path::Path;

use anyhow::Result;

use super::file_rule::CompiledFileRule;
use super::{BaseFlags, FileRule, LimitPatch, Limits};
use crate::config::glob::NamePatterns;
use crate::config::keys::LimitKey;
use crate::config::patterns::LinePatterns;
use crate::config::test_templates::TestTemplates;

/// Resolves the limits for a path: defaults, then matching `[[files]]` rules in order,
/// then CLI flags.
pub struct LimitRules {
    rules: Vec<CompiledFileRule>,
    base: BaseFlags,
    cli: LimitPatch,
}

impl LimitRules {
    #[expect(
        clippy::large_types_passed_by_value,
        reason = "built once per run from config and CLI flags"
    )]
    pub fn new(files: &[FileRule], cli: LimitPatch) -> Result<Self> {
        Self::with_base(files, BaseFlags::default(), cli)
    }

    #[expect(
        clippy::large_types_passed_by_value,
        reason = "built once per run from config and CLI flags"
    )]
    pub fn with_base(files: &[FileRule], base: BaseFlags, cli: LimitPatch) -> Result<Self> {
        let rules = files.iter().map(FileRule::compile).collect::<Result<_>>()?;
        Ok(Self { rules, base, cli })
    }

    pub fn for_path(&self, path: &Path) -> Limits {
        let mut limits = Limits {
            flags: self.base,
            ..Limits::default()
        };
        for rule in &self.rules {
            if rule.matcher.is_match(path) {
                limits.apply(&rule.patch);
            }
        }
        limits.apply(&self.cli);
        limits
    }

    pub fn forbid_imports_for(&self, path: &Path) -> Option<&NamePatterns> {
        self.last_set(path, |r| r.forbid_imports.as_ref())
    }

    pub fn allow_imports_for(&self, path: &Path) -> Option<&NamePatterns> {
        self.last_set(path, |r| r.allow_imports.as_ref())
    }

    pub fn forbid_comments_for(&self, path: &Path) -> Option<&NamePatterns> {
        self.last_set(path, |r| r.forbid_comments.as_ref())
    }

    /// `require_test` of the last matching rule that sets it; `None` when empty.
    pub fn require_test_for(&self, path: &Path) -> Option<&TestTemplates> {
        self.last_set(path, |r| r.require_test.as_ref())
            .filter(|t| !t.is_empty())
    }

    /// `forbid_patterns` of the last matching rule that sets it; `None` when empty.
    pub fn forbid_patterns_for(&self, path: &Path) -> Option<&LinePatterns> {
        self.last_set(path, |r| r.forbid_patterns.as_ref())
            .filter(|p| !p.is_empty())
    }

    /// Whether a `[[files]]` rule sets `allow_imports`.
    pub fn sets_allow_imports(&self) -> bool {
        self.rules.iter().any(|r| r.allow_imports.is_some())
    }

    /// Whether a `[[files]]` rule or the CLI sets a graph limit (fan-in/out, context tokens).
    pub fn sets_graph_limits(&self) -> bool {
        self.rules.iter().any(|r| r.patch.sets_graph_limits()) || self.cli.sets_graph_limits()
    }

    pub fn governing_rule(&self, path: &Path, key: LimitKey) -> Option<usize> {
        self.rules
            .iter()
            .rposition(|r| r.patch.get(key).is_some() && r.matcher.is_match(path))
    }

    /// The field `get` reads, from the last rule that sets it and matches `path`.
    fn last_set<T>(
        &self,
        path: &Path,
        get: impl Fn(&CompiledFileRule) -> Option<&T>,
    ) -> Option<&T> {
        self.rules
            .iter()
            .rev()
            .filter_map(|r| Some((r, get(r)?)))
            .find(|(r, _)| r.matcher.is_match(path))
            .map(|(_, value)| value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Depth, Lines, Tokens};

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
        assert_eq!(src.func.lines, Lines(60));
        assert_eq!(src.func.depth, Some(Depth(4)));
        assert_eq!(src.file_lines, Lines(300));
        assert_eq!(src.file_tokens, Tokens(8000));

        let test = rules.for_path(Path::new("tests/cli.rs"));
        assert_eq!(test.func.lines, Lines(200));
        assert_eq!(test.func.depth, Some(Depth(4)));
    }
}
