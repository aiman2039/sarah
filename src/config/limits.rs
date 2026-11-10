use std::path::Path;

use anyhow::{Context, Result};
use globset::GlobMatcher;
use serde::Deserialize;

use super::glob::{GlobHelpers, NamePatterns};
use super::keys::LimitKey;
use crate::units::{
    Cognitive, Cyclomatic, Depth, Fields, Functions, Lines, Methods, Params, Tokens,
};

/// Size and lint limits that apply to one file, its functions and its types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub file_tokens: Tokens,
    pub file_lines: Lines,
    pub file_fns: Option<Functions>,
    pub fn_lines: Lines,
    pub fn_tokens: Option<Tokens>,
    pub fn_depth: Option<Depth>,
    pub fn_params: Option<Params>,
    pub fn_bool_params: Option<Params>,
    pub fn_cyclomatic: Option<Cyclomatic>,
    pub fn_cognitive: Option<Cognitive>,
    pub type_fields: Option<Fields>,
    pub type_methods: Option<Methods>,
    pub unused_allow: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            file_tokens: Tokens(8000),
            file_lines: Lines(500),
            file_fns: None,
            fn_lines: Lines(80),
            fn_tokens: None,
            fn_depth: None,
            fn_params: None,
            fn_bool_params: None,
            fn_cyclomatic: None,
            fn_cognitive: None,
            type_fields: None,
            type_methods: None,
            unused_allow: false,
        }
    }
}

/// Limits that a rule or the CLI sets; `None` keeps the earlier value.
#[derive(Clone, Copy, Debug, Default)]
pub struct LimitPatch {
    pub max_file_tokens: Option<Tokens>,
    pub max_file_lines: Option<Lines>,
    pub max_file_fns: Option<Functions>,
    pub max_fn_lines: Option<Lines>,
    pub max_fn_tokens: Option<Tokens>,
    pub max_fn_depth: Option<Depth>,
    pub max_fn_params: Option<Params>,
    pub max_fn_bool_params: Option<Params>,
    pub max_fn_cyclomatic: Option<Cyclomatic>,
    pub max_fn_cognitive: Option<Cognitive>,
    pub max_type_fields: Option<Fields>,
    pub max_type_methods: Option<Methods>,
    pub unused_allow: Option<bool>,
}

/// A `[[files]]` entry in `sarah.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRule {
    pub path: String,
    pub max_file_tokens: Option<Tokens>,
    pub max_file_lines: Option<Lines>,
    pub max_file_fns: Option<Functions>,
    pub max_fn_lines: Option<Lines>,
    pub max_fn_tokens: Option<Tokens>,
    pub max_fn_depth: Option<Depth>,
    pub max_fn_params: Option<Params>,
    pub max_fn_bool_params: Option<Params>,
    pub max_fn_cyclomatic: Option<Cyclomatic>,
    pub max_fn_cognitive: Option<Cognitive>,
    pub max_type_fields: Option<Fields>,
    pub max_type_methods: Option<Methods>,
    pub unused_allow: Option<bool>,
    pub forbid_imports: Option<Vec<String>>,
}

struct CompiledFileRule {
    matcher: GlobMatcher,
    patch: LimitPatch,
    forbid_imports: Option<NamePatterns>,
}

/// Resolves the limits for a path: defaults, then matching `[[files]]` rules in order,
/// then CLI flags.
pub struct LimitRules {
    rules: Vec<CompiledFileRule>,
    base_unused_allow: bool,
    cli: LimitPatch,
}

impl Limits {
    fn apply(&mut self, patch: &LimitPatch) {
        self.file_tokens = patch.max_file_tokens.unwrap_or(self.file_tokens);
        self.file_lines = patch.max_file_lines.unwrap_or(self.file_lines);
        self.file_fns = patch.max_file_fns.or(self.file_fns);
        self.fn_lines = patch.max_fn_lines.unwrap_or(self.fn_lines);
        self.fn_tokens = patch.max_fn_tokens.or(self.fn_tokens);
        self.fn_depth = patch.max_fn_depth.or(self.fn_depth);
        self.fn_params = patch.max_fn_params.or(self.fn_params);
        self.fn_bool_params = patch.max_fn_bool_params.or(self.fn_bool_params);
        self.fn_cyclomatic = patch.max_fn_cyclomatic.or(self.fn_cyclomatic);
        self.fn_cognitive = patch.max_fn_cognitive.or(self.fn_cognitive);
        self.type_fields = patch.max_type_fields.or(self.type_fields);
        self.type_methods = patch.max_type_methods.or(self.type_methods);
        self.unused_allow = patch.unused_allow.unwrap_or(self.unused_allow);
    }
}

impl LimitPatch {
    pub fn get(&self, key: LimitKey) -> Option<usize> {
        match key {
            LimitKey::FileTokens => self.max_file_tokens.map(|v| v.0),
            LimitKey::FileLines => self.max_file_lines.map(|v| v.0),
            LimitKey::FileFns => self.max_file_fns.map(|v| v.0),
            LimitKey::FnLines => self.max_fn_lines.map(|v| v.0),
            LimitKey::FnTokens => self.max_fn_tokens.map(|v| v.0),
            LimitKey::FnDepth => self.max_fn_depth.map(|v| v.0),
            LimitKey::FnParams => self.max_fn_params.map(|v| v.0),
            LimitKey::FnBoolParams => self.max_fn_bool_params.map(|v| v.0),
            LimitKey::FnCyclomatic => self.max_fn_cyclomatic.map(|v| v.0),
            LimitKey::FnCognitive => self.max_fn_cognitive.map(|v| v.0),
            LimitKey::TypeFields => self.max_type_fields.map(|v| v.0),
            LimitKey::TypeMethods => self.max_type_methods.map(|v| v.0),
        }
    }
}

impl FileRule {
    fn compile(&self) -> Result<CompiledFileRule> {
        let ctx = || format!("in [[files]] path = {:?} of sarah.toml", self.path);
        let matcher = GlobHelpers::path_matcher(&self.path).with_context(ctx)?;
        let forbid_imports = self
            .forbid_imports
            .as_deref()
            .map(NamePatterns::new)
            .transpose()
            .with_context(ctx)?;
        Ok(CompiledFileRule {
            matcher,
            patch: self.patch(),
            forbid_imports,
        })
    }

    fn patch(&self) -> LimitPatch {
        LimitPatch {
            max_file_tokens: self.max_file_tokens,
            max_file_lines: self.max_file_lines,
            max_file_fns: self.max_file_fns,
            max_fn_lines: self.max_fn_lines,
            max_fn_tokens: self.max_fn_tokens,
            max_fn_depth: self.max_fn_depth,
            max_fn_params: self.max_fn_params,
            max_fn_bool_params: self.max_fn_bool_params,
            max_fn_cyclomatic: self.max_fn_cyclomatic,
            max_fn_cognitive: self.max_fn_cognitive,
            max_type_fields: self.max_type_fields,
            max_type_methods: self.max_type_methods,
            unused_allow: self.unused_allow,
        }
    }
}

impl LimitRules {
    pub fn new(files: &[FileRule], cli: LimitPatch) -> Result<Self> {
        Self::with_unused_allow(files, false, cli)
    }

    pub fn with_unused_allow(
        files: &[FileRule],
        base_unused_allow: bool,
        cli: LimitPatch,
    ) -> Result<Self> {
        let rules = files.iter().map(FileRule::compile).collect::<Result<_>>()?;
        Ok(Self {
            rules,
            base_unused_allow,
            cli,
        })
    }

    pub fn for_path(&self, path: &Path) -> Limits {
        let mut limits = Limits {
            unused_allow: self.base_unused_allow,
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

    /// Last matching `[[files]]` rule's `forbid_imports` patterns, if non-empty.
    pub fn forbid_imports_for(&self, path: &Path) -> Option<&NamePatterns> {
        let mut active: Option<&NamePatterns> = None;
        for rule in &self.rules {
            if rule.matcher.is_match(path)
                && let Some(patterns) = &rule.forbid_imports
            {
                active = Some(patterns);
            }
        }
        active
    }

    /// Index of the last `[[files]]` rule that matches `path` and sets `key` (CLI flags
    /// ignored): the rule whose value applies to the path.
    pub fn governing_rule(&self, path: &Path, key: LimitKey) -> Option<usize> {
        self.rules
            .iter()
            .rposition(|r| r.patch.get(key).is_some() && r.matcher.is_match(path))
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
