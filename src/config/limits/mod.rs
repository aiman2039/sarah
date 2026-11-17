//! Per-path limits: defaults, `[[files]]` rules and CLI flags, merged field by field.

mod analysis;
mod file_rule;
mod patch;
mod rules;

pub use analysis::{AnalysisKey, AnalysisRules};
pub use file_rule::FileRule;
pub use patch::{FlagPatch, FnPatch, LimitPatch};
pub use rules::LimitRules;

use crate::units::Fan;
use crate::units::{Chars, Items, Percent, Returns, Types};
use crate::units::{
    Cognitive, Cyclomatic, Depth, Fields, Functions, Imports, Lines, Methods, Params, Tokens,
};

/// Size and lint limits that apply to one file, its functions and its types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub file_tokens: Tokens,
    pub file_lines: Lines,
    pub file_fns: Option<Functions>,
    pub file_imports: Option<Imports>,
    pub file_types: Option<Types>,
    pub public_items: Option<Items>,
    pub line_length: Option<Chars>,
    pub min_comment_ratio: Option<Percent>,
    pub max_comment_ratio: Option<Percent>,
    pub func: FnLimits,
    pub type_fields: Option<Fields>,
    pub type_methods: Option<Methods>,
    pub todos: Option<usize>,
    pub fan_in: Option<Fan>,
    pub fan_out: Option<Fan>,
    pub context_tokens: Option<Tokens>,
    pub flags: BaseFlags,
}

/// Limits that apply to each function of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FnLimits {
    pub lines: Lines,
    pub tokens: Option<Tokens>,
    pub depth: Option<Depth>,
    pub params: Option<Params>,
    pub bool_params: Option<Params>,
    pub cyclomatic: Option<Cyclomatic>,
    pub cognitive: Option<Cognitive>,
    pub returns: Option<Returns>,
}

/// On/off switches: top-level defaults from `sarah.toml` before per-path `[[files]]` rules
/// apply, and the resolved switches of one path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaseFlags {
    pub unused_allow: bool,
    pub require_allow_reason: bool,
    pub exclude_inline_tests: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            file_tokens: Tokens(8000),
            file_lines: Lines(500),
            file_fns: None,
            file_imports: None,
            file_types: None,
            public_items: None,
            line_length: None,
            min_comment_ratio: None,
            max_comment_ratio: None,
            func: FnLimits::default(),
            type_fields: None,
            type_methods: None,
            todos: None,
            fan_in: None,
            fan_out: None,
            context_tokens: None,
            flags: BaseFlags::default(),
        }
    }
}

impl Default for FnLimits {
    fn default() -> Self {
        Self {
            lines: Lines(80),
            tokens: None,
            depth: None,
            params: None,
            bool_params: None,
            cyclomatic: None,
            cognitive: None,
            returns: None,
        }
    }
}

impl Limits {
    fn apply(&mut self, patch: &LimitPatch) {
        self.file_tokens = patch.max_file_tokens.unwrap_or(self.file_tokens);
        self.file_lines = patch.max_file_lines.unwrap_or(self.file_lines);
        self.file_fns = patch.max_file_fns.or(self.file_fns);
        self.file_imports = patch.max_file_imports.or(self.file_imports);
        self.file_types = patch.max_file_types.or(self.file_types);
        self.public_items = patch.max_public_items.or(self.public_items);
        self.line_length = patch.max_line_length.or(self.line_length);
        self.min_comment_ratio = patch.min_comment_ratio.or(self.min_comment_ratio);
        self.max_comment_ratio = patch.max_comment_ratio.or(self.max_comment_ratio);
        self.func.apply(&patch.func);
        self.type_fields = patch.max_type_fields.or(self.type_fields);
        self.type_methods = patch.max_type_methods.or(self.type_methods);
        self.todos = patch.max_todos.or(self.todos);
        self.fan_in = patch.max_fan_in.or(self.fan_in);
        self.fan_out = patch.max_fan_out.or(self.fan_out);
        self.context_tokens = patch.max_context_tokens.or(self.context_tokens);
        self.flags.apply(&patch.flags);
    }
}

impl FnLimits {
    fn apply(&mut self, patch: &FnPatch) {
        self.lines = patch.lines.unwrap_or(self.lines);
        self.tokens = patch.tokens.or(self.tokens);
        self.depth = patch.depth.or(self.depth);
        self.params = patch.params.or(self.params);
        self.bool_params = patch.bool_params.or(self.bool_params);
        self.cyclomatic = patch.cyclomatic.or(self.cyclomatic);
        self.cognitive = patch.cognitive.or(self.cognitive);
        self.returns = patch.returns.or(self.returns);
    }
}

impl BaseFlags {
    fn apply(&mut self, patch: &FlagPatch) {
        self.unused_allow = patch.unused_allow.unwrap_or(self.unused_allow);
        self.require_allow_reason = patch
            .require_allow_reason
            .unwrap_or(self.require_allow_reason);
        self.exclude_inline_tests = patch
            .exclude_inline_tests
            .unwrap_or(self.exclude_inline_tests);
    }
}
