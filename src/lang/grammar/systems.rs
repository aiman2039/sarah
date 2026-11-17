//! Systems languages: Rust, Go, C, C++, Zig.

use super::{Grammar, ParamSpec};

pub(super) const RUST: Grammar = Grammar {
    nesting: &[
        "if_expression",
        "match_expression",
        "for_expression",
        "while_expression",
        "loop_expression",
    ],
    branches: &[
        "if_expression",
        "for_expression",
        "while_expression",
        "loop_expression",
    ],
    elifs: &[],
    switches: &["match_expression"],
    cases: &["match_arm"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["parameters"],
        items: &["parameter", "variadic_parameter"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const GO: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "for_statement",
        "expression_switch_statement",
        "type_switch_statement",
        "select_statement",
    ],
    branches: &["if_statement", "for_statement"],
    elifs: &[],
    switches: &[
        "expression_switch_statement",
        "type_switch_statement",
        "select_statement",
    ],
    cases: &["expression_case", "type_case", "communication_case"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["parameter_list"],
        items: &["parameter_declaration", "variadic_parameter_declaration"],
        names_per_item: true,
    }),
    keyword_calls: &[],
};

pub(super) const C: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "for_statement",
        "while_statement",
        "do_statement",
        "switch_statement",
    ],
    branches: &[
        "if_statement",
        "for_statement",
        "while_statement",
        "do_statement",
        "conditional_expression",
    ],
    elifs: &[],
    switches: &["switch_statement"],
    cases: &["case_statement"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["parameter_list"],
        items: &["parameter_declaration", "variadic_parameter"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const CPP: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "for_statement",
        "for_range_loop",
        "while_statement",
        "do_statement",
        "switch_statement",
        "try_statement",
    ],
    branches: &[
        "if_statement",
        "for_statement",
        "for_range_loop",
        "while_statement",
        "do_statement",
        "catch_clause",
        "conditional_expression",
    ],
    params: Some(ParamSpec {
        lists: &["parameter_list"],
        items: &[
            "parameter_declaration",
            "optional_parameter_declaration",
            "variadic_parameter_declaration",
        ],
        names_per_item: false,
    }),
    ..C
};

pub(super) const ZIG: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "if_expression",
        "for_statement",
        "for_expression",
        "while_statement",
        "while_expression",
        "switch_expression",
    ],
    branches: &[
        "if_statement",
        "if_expression",
        "for_statement",
        "for_expression",
        "while_statement",
        "while_expression",
        "catch_expression",
    ],
    elifs: &[],
    switches: &["switch_expression"],
    cases: &["switch_case"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["parameters"],
        items: &["parameter"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};
