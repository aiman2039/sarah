use anyhow::{Context, Result};
use globset::GlobMatcher;
use serde::Deserialize;

use super::{FlagPatch, FnPatch, LimitPatch};
use crate::config::glob::{GlobHelpers, NamePatterns};
use crate::config::patterns::LinePatterns;
use crate::config::test_templates::TestTemplates;
use crate::units::Fan;
use crate::units::{Chars, Items, Percent, Returns, Types};
use crate::units::{
    Cognitive, Cyclomatic, Depth, Fields, Functions, Imports, Lines, Methods, Params, Tokens,
};

// sarah-allow(large_type): mirrors the flat `[[files]]` keys; `deny_unknown_fields` forbids flatten
/// A `[[files]]` entry in `sarah.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRule {
    pub path: String,
    pub max_file_tokens: Option<Tokens>,
    pub max_file_lines: Option<Lines>,
    pub max_file_fns: Option<Functions>,
    pub max_file_imports: Option<Imports>,
    pub max_file_types: Option<Types>,
    pub max_public_items: Option<Items>,
    pub max_line_length: Option<Chars>,
    pub min_comment_ratio: Option<Percent>,
    pub max_comment_ratio: Option<Percent>,
    pub max_fn_lines: Option<Lines>,
    pub max_fn_tokens: Option<Tokens>,
    pub max_fn_depth: Option<Depth>,
    pub max_fn_params: Option<Params>,
    pub max_fn_bool_params: Option<Params>,
    pub max_fn_cyclomatic: Option<Cyclomatic>,
    pub max_fn_cognitive: Option<Cognitive>,
    pub max_fn_returns: Option<Returns>,
    pub max_type_fields: Option<Fields>,
    pub max_type_methods: Option<Methods>,
    pub max_todos: Option<usize>,
    pub max_fan_in: Option<Fan>,
    pub max_fan_out: Option<Fan>,
    pub max_context_tokens: Option<Tokens>,
    pub unused_allow: Option<bool>,
    pub require_allow_reason: Option<bool>,
    pub exclude_inline_tests: Option<bool>,
    pub forbid_imports: Option<Vec<String>>,
    pub allow_imports: Option<Vec<String>>,
    pub forbid_comments: Option<Vec<String>>,
    pub require_test: Option<Vec<String>>,
    pub forbid_patterns: Option<Vec<String>>,
}

/// A `FileRule` with its globs and patterns compiled.
pub(super) struct CompiledFileRule {
    pub matcher: GlobMatcher,
    pub patch: LimitPatch,
    pub forbid_imports: Option<NamePatterns>,
    pub allow_imports: Option<NamePatterns>,
    pub forbid_comments: Option<NamePatterns>,
    pub require_test: Option<TestTemplates>,
    pub forbid_patterns: Option<LinePatterns>,
}

impl FileRule {
    pub(super) fn compile(&self) -> Result<CompiledFileRule> {
        let ctx = || format!("in [[files]] path = {:?} of sarah.toml", self.path);
        let matcher = GlobHelpers::path_matcher(&self.path).with_context(ctx)?;
        let patterns = |globs: &Option<Vec<String>>| {
            globs
                .as_deref()
                .map(NamePatterns::new)
                .transpose()
                .with_context(ctx)
        };
        Ok(CompiledFileRule {
            matcher,
            patch: self.patch(),
            forbid_imports: patterns(&self.forbid_imports)?,
            allow_imports: patterns(&self.allow_imports)?,
            forbid_comments: patterns(&self.forbid_comments)?,
            require_test: (self.require_test.as_deref().map(TestTemplates::new))
                .transpose()
                .with_context(ctx)?,
            forbid_patterns: (self.forbid_patterns.as_deref().map(LinePatterns::new))
                .transpose()
                .with_context(ctx)?,
        })
    }

    fn patch(&self) -> LimitPatch {
        LimitPatch {
            max_file_tokens: self.max_file_tokens,
            max_file_lines: self.max_file_lines,
            max_file_fns: self.max_file_fns,
            max_file_imports: self.max_file_imports,
            max_file_types: self.max_file_types,
            max_public_items: self.max_public_items,
            max_line_length: self.max_line_length,
            min_comment_ratio: self.min_comment_ratio,
            max_comment_ratio: self.max_comment_ratio,
            func: self.fn_patch(),
            max_type_fields: self.max_type_fields,
            max_type_methods: self.max_type_methods,
            max_todos: self.max_todos,
            max_fan_in: self.max_fan_in,
            max_fan_out: self.max_fan_out,
            max_context_tokens: self.max_context_tokens,
            flags: FlagPatch {
                unused_allow: self.unused_allow,
                require_allow_reason: self.require_allow_reason,
                exclude_inline_tests: self.exclude_inline_tests,
            },
        }
    }

    fn fn_patch(&self) -> FnPatch {
        FnPatch {
            lines: self.max_fn_lines,
            tokens: self.max_fn_tokens,
            depth: self.max_fn_depth,
            params: self.max_fn_params,
            bool_params: self.max_fn_bool_params,
            cyclomatic: self.max_fn_cyclomatic,
            cognitive: self.max_fn_cognitive,
            returns: self.max_fn_returns,
        }
    }
}
