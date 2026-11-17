use clap::Args as ClapArgs;

use crate::config::keys::LimitKey;
use crate::units::Fan;
use crate::units::{Chars, Items, Percent, Returns, Types};
use crate::units::{
    Cognitive, Cyclomatic, Depth, Fields, Functions, Imports, Lines, Methods, Params, Tokens,
};

/// Limits that a rule or the CLI sets; `None` keeps the earlier value.
#[derive(Clone, Copy, Debug, Default)]
pub struct LimitPatch {
    pub max_file_tokens: Option<Tokens>,
    pub max_file_lines: Option<Lines>,
    pub max_file_fns: Option<Functions>,
    pub max_file_imports: Option<Imports>,
    pub max_file_types: Option<Types>,
    pub max_public_items: Option<Items>,
    pub max_line_length: Option<Chars>,
    pub min_comment_ratio: Option<Percent>,
    pub max_comment_ratio: Option<Percent>,
    pub func: FnPatch,
    pub max_type_fields: Option<Fields>,
    pub max_type_methods: Option<Methods>,
    pub max_todos: Option<usize>,
    pub max_fan_in: Option<Fan>,
    pub max_fan_out: Option<Fan>,
    pub max_context_tokens: Option<Tokens>,
    pub flags: FlagPatch,
}

/// The `max_fn_*` limits of a `LimitPatch`; also the `--max-fn-*` CLI flags.
#[derive(ClapArgs, Clone, Copy, Debug, Default)]
pub struct FnPatch {
    /// [default: 80]
    #[arg(id = "fn_lines", long = "max-fn-lines")]
    pub lines: Option<Lines>,

    /// Warn on functions over this many tokens [default: off]
    #[arg(id = "fn_tokens", long = "max-fn-tokens")]
    pub tokens: Option<Tokens>,

    /// Warn on functions nested deeper than this (if/loop/match levels) [default: off]
    #[arg(id = "fn_depth", long = "max-fn-depth")]
    pub depth: Option<Depth>,

    /// Warn on functions with more parameters than this (`self` excluded) [default: off]
    #[arg(id = "fn_params", long = "max-fn-params")]
    pub params: Option<Params>,

    /// Warn on functions with more boolean parameters than this [default: off]
    #[arg(id = "fn_bool_params", long = "max-fn-bool-params")]
    pub bool_params: Option<Params>,

    /// Warn on functions over this cyclomatic complexity [default: off]
    #[arg(id = "fn_cyclomatic", long = "max-fn-cyclomatic")]
    pub cyclomatic: Option<Cyclomatic>,

    /// Warn on functions over this cognitive complexity [default: off]
    #[arg(id = "fn_cognitive", long = "max-fn-cognitive")]
    pub cognitive: Option<Cognitive>,

    /// Warn on functions with more explicit `return` statements than this [default: off]
    #[arg(id = "fn_returns", long = "max-fn-returns")]
    pub returns: Option<Returns>,
}

/// The on/off switches of a `LimitPatch`.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlagPatch {
    pub unused_allow: Option<bool>,
    pub require_allow_reason: Option<bool>,
    pub exclude_inline_tests: Option<bool>,
}

impl LimitPatch {
    /// Whether this patch sets `max_fan_in`, `max_fan_out` or `max_context_tokens`.
    pub fn sets_graph_limits(&self) -> bool {
        self.max_fan_in.is_some() || self.max_fan_out.is_some() || self.max_context_tokens.is_some()
    }

    pub fn get(&self, key: LimitKey) -> Option<usize> {
        match key {
            LimitKey::FileTokens => self.max_file_tokens.map(|v| v.0),
            LimitKey::FileLines => self.max_file_lines.map(|v| v.0),
            LimitKey::FileFns => self.max_file_fns.map(|v| v.0),
            LimitKey::FileImports => self.max_file_imports.map(|v| v.0),
            LimitKey::FileTypes => self.max_file_types.map(|v| v.0),
            LimitKey::PublicItems => self.max_public_items.map(|v| v.0),
            LimitKey::LineLength => self.max_line_length.map(|v| v.0),
            LimitKey::FnLines => self.func.lines.map(|v| v.0),
            LimitKey::FnTokens => self.func.tokens.map(|v| v.0),
            LimitKey::FnDepth => self.func.depth.map(|v| v.0),
            LimitKey::FnParams => self.func.params.map(|v| v.0),
            LimitKey::FnBoolParams => self.func.bool_params.map(|v| v.0),
            LimitKey::FnCyclomatic => self.func.cyclomatic.map(|v| v.0),
            LimitKey::FnCognitive => self.func.cognitive.map(|v| v.0),
            LimitKey::FnReturns => self.func.returns.map(|v| v.0),
            LimitKey::TypeFields => self.max_type_fields.map(|v| v.0),
            LimitKey::TypeMethods => self.max_type_methods.map(|v| v.0),
        }
    }
}
