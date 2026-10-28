//! Scripting languages: Python, JavaScript/TypeScript, Ruby, PHP, Lua, Elixir, Bash.

use super::{Grammar, ParamSpec};

pub(super) const PYTHON: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "for_statement",
        "while_statement",
        "try_statement",
        "with_statement",
        "match_statement",
    ],
    branches: &[
        "if_statement",
        "for_statement",
        "while_statement",
        "except_clause",
        "conditional_expression",
    ],
    elifs: &["elif_clause"],
    switches: &["match_statement"],
    cases: &["case_clause"],
    binaries: &["boolean_operator"],
    params: Some(ParamSpec {
        lists: &["parameters"],
        items: &[
            "identifier",
            "typed_parameter",
            "default_parameter",
            "typed_default_parameter",
            "list_splat_pattern",
            "dictionary_splat_pattern",
        ],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

const JS_NESTING: &[&str] = &[
    "if_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
    "switch_statement",
    "try_statement",
];

const JS_BRANCHES: &[&str] = &[
    "if_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
    "catch_clause",
    "ternary_expression",
];

pub(super) const JAVASCRIPT: Grammar = Grammar {
    nesting: JS_NESTING,
    branches: JS_BRANCHES,
    elifs: &[],
    switches: &["switch_statement"],
    cases: &["switch_case"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["formal_parameters"],
        items: &[
            "identifier",
            "assignment_pattern",
            "rest_pattern",
            "object_pattern",
            "array_pattern",
        ],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const TYPESCRIPT: Grammar = Grammar {
    params: Some(ParamSpec {
        lists: &["formal_parameters"],
        items: &["required_parameter", "optional_parameter"],
        names_per_item: false,
    }),
    ..JAVASCRIPT
};

pub(super) const RUBY: Grammar = Grammar {
    nesting: &["if", "unless", "while", "until", "for", "case", "begin"],
    branches: &[
        "if",
        "unless",
        "while",
        "until",
        "for",
        "rescue",
        "conditional",
        "if_modifier",
        "unless_modifier",
        "while_modifier",
        "until_modifier",
    ],
    elifs: &["elsif"],
    switches: &["case"],
    cases: &["when"],
    binaries: &["binary"],
    params: Some(ParamSpec {
        lists: &["method_parameters"],
        items: &[
            "identifier",
            "optional_parameter",
            "splat_parameter",
            "hash_splat_parameter",
            "block_parameter",
            "keyword_parameter",
        ],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const PHP: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "for_statement",
        "foreach_statement",
        "while_statement",
        "do_statement",
        "switch_statement",
        "try_statement",
    ],
    branches: &[
        "if_statement",
        "for_statement",
        "foreach_statement",
        "while_statement",
        "do_statement",
        "catch_clause",
        "conditional_expression",
    ],
    elifs: &["else_if_clause"],
    switches: &["switch_statement", "match_expression"],
    cases: &["case_statement", "match_conditional_expression"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["formal_parameters"],
        items: &[
            "simple_parameter",
            "variadic_parameter",
            "property_promotion_parameter",
        ],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const LUA: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "for_statement",
        "while_statement",
        "repeat_statement",
    ],
    branches: &[
        "if_statement",
        "for_statement",
        "while_statement",
        "repeat_statement",
    ],
    elifs: &["elseif_statement"],
    switches: &[],
    cases: &[],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["parameters"],
        items: &["identifier", "vararg_expression"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const ELIXIR: Grammar = Grammar {
    nesting: &[
        "if", "unless", "case", "cond", "for", "with", "try", "receive",
    ],
    branches: &["if", "unless", "for", "with", "rescue_block", "catch_block"],
    elifs: &[],
    switches: &["case", "cond", "receive"],
    cases: &[],
    binaries: &["binary_operator"],
    params: None,
    keyword_calls: &[
        "if", "unless", "case", "cond", "for", "with", "try", "receive",
    ],
};

pub(super) const BASH: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "for_statement",
        "c_style_for_statement",
        "while_statement",
        "case_statement",
    ],
    branches: &[
        "if_statement",
        "for_statement",
        "c_style_for_statement",
        "while_statement",
        "ternary_expression",
    ],
    elifs: &["elif_clause"],
    switches: &["case_statement"],
    cases: &["case_item"],
    binaries: &["binary_expression", "list"],
    params: None,
    keyword_calls: &[],
};
