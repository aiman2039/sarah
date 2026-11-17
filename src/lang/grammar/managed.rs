//! JVM, .NET and Apple languages: Java, C#, Kotlin, Swift, Scala.

use super::{Grammar, ParamSpec};

pub(super) const JAVA: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "for_statement",
        "enhanced_for_statement",
        "while_statement",
        "do_statement",
        "switch_expression",
        "try_statement",
    ],
    branches: &[
        "if_statement",
        "for_statement",
        "enhanced_for_statement",
        "while_statement",
        "do_statement",
        "catch_clause",
        "ternary_expression",
    ],
    elifs: &[],
    switches: &["switch_expression"],
    cases: &["switch_block_statement_group", "switch_rule"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["formal_parameters"],
        items: &["formal_parameter", "spread_parameter"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const CSHARP: Grammar = Grammar {
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
    elifs: &[],
    switches: &["switch_statement", "switch_expression"],
    cases: &["switch_section", "switch_expression_arm"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["parameter_list"],
        items: &["parameter"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const KOTLIN: Grammar = Grammar {
    nesting: &[
        "if_expression",
        "when_expression",
        "for_statement",
        "while_statement",
        "do_while_statement",
        "try_expression",
    ],
    branches: &[
        "if_expression",
        "for_statement",
        "while_statement",
        "do_while_statement",
        "catch_block",
    ],
    elifs: &[],
    switches: &["when_expression"],
    cases: &["when_entry"],
    binaries: &["binary_expression"],
    params: Some(ParamSpec {
        lists: &["function_value_parameters"],
        items: &["parameter"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const SWIFT: Grammar = Grammar {
    nesting: &[
        "if_statement",
        "guard_statement",
        "for_statement",
        "while_statement",
        "repeat_while_statement",
        "switch_statement",
        "do_statement",
    ],
    branches: &[
        "if_statement",
        "guard_statement",
        "for_statement",
        "while_statement",
        "repeat_while_statement",
        "catch_block",
        "ternary_expression",
    ],
    elifs: &[],
    switches: &["switch_statement"],
    cases: &["switch_entry"],
    binaries: &["conjunction_expression", "disjunction_expression"],
    params: Some(ParamSpec {
        lists: &[],
        items: &["parameter"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};

pub(super) const SCALA: Grammar = Grammar {
    nesting: &[
        "if_expression",
        "match_expression",
        "for_expression",
        "while_expression",
        "do_while_expression",
        "try_expression",
    ],
    branches: &[
        "if_expression",
        "for_expression",
        "while_expression",
        "do_while_expression",
        "catch_clause",
    ],
    elifs: &[],
    switches: &["match_expression"],
    cases: &["case_clause"],
    binaries: &["infix_expression"],
    params: Some(ParamSpec {
        lists: &["parameters"],
        items: &["parameter"],
        names_per_item: false,
    }),
    keyword_calls: &[],
};
