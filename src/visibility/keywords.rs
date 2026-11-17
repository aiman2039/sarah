//! Languages with visibility keywords: Rust, Java, C#, Kotlin, Swift, Scala, PHP, Zig.

use tree_sitter::Node;

use super::{VisibilityHelpers, VisibilitySpec};
use crate::lang::Scope;

const HIDDEN: &[&str] = &["private", "protected", "internal"];

/// Bare `pub`; `pub(crate)`, `pub(super)` and `pub(in ..)` are not public API.
pub(super) const RUST: VisibilitySpec = VisibilitySpec {
    items: &[
        "function_item",
        "struct_item",
        "enum_item",
        "union_item",
        "trait_item",
        "type_item",
    ],
    opaque: &["function_item"],
    count: KeywordHelpers::rust,
};

/// `public`; members of an interface are public unless `private`.
pub(super) const JAVA: VisibilitySpec = VisibilitySpec {
    items: &[
        "class_declaration",
        "interface_declaration",
        "enum_declaration",
        "record_declaration",
        "annotation_type_declaration",
        "method_declaration",
        "constructor_declaration",
    ],
    opaque: &[
        "method_declaration",
        "constructor_declaration",
        "field_declaration",
    ],
    count: KeywordHelpers::java,
};

/// `public`; members of an interface are public unless `private`.
pub(super) const CSHARP: VisibilitySpec = VisibilitySpec {
    items: &[
        "class_declaration",
        "struct_declaration",
        "interface_declaration",
        "enum_declaration",
        "record_declaration",
        "delegate_declaration",
        "method_declaration",
        "constructor_declaration",
    ],
    opaque: &[
        "method_declaration",
        "constructor_declaration",
        "property_declaration",
        "field_declaration",
    ],
    count: KeywordHelpers::csharp,
};

/// Public unless `private`, `protected` or `internal`.
pub(super) const KOTLIN: VisibilitySpec = VisibilitySpec {
    items: &[
        "class_declaration",
        "object_declaration",
        "function_declaration",
        "type_alias",
    ],
    opaque: &["function_declaration", "property_declaration"],
    count: KeywordHelpers::not_hidden,
};

/// Only an explicit `public` or `open` (the default is `internal`).
pub(super) const SWIFT: VisibilitySpec = VisibilitySpec {
    items: &[
        "class_declaration",
        "protocol_declaration",
        "function_declaration",
        "init_declaration",
        "typealias_declaration",
    ],
    opaque: &[
        "function_declaration",
        "init_declaration",
        "property_declaration",
    ],
    count: KeywordHelpers::swift,
};

/// Public unless `private` or `protected`.
pub(super) const SCALA: VisibilitySpec = VisibilitySpec {
    items: &[
        "class_definition",
        "object_definition",
        "trait_definition",
        "enum_definition",
        "function_definition",
        "function_declaration",
        "type_definition",
    ],
    opaque: &["function_definition", "val_definition", "var_definition"],
    count: KeywordHelpers::not_hidden,
};

/// Functions and types are always public; methods unless `private` or `protected`.
pub(super) const PHP: VisibilitySpec = VisibilitySpec {
    items: &[
        "function_definition",
        "class_declaration",
        "interface_declaration",
        "trait_declaration",
        "enum_declaration",
        "method_declaration",
    ],
    opaque: &["function_definition", "method_declaration"],
    count: KeywordHelpers::not_hidden,
};

/// `pub fn` and `pub const X = struct/enum/union/opaque`.
pub(super) const ZIG: VisibilitySpec = VisibilitySpec {
    items: &["function_declaration", "variable_declaration"],
    opaque: &["function_declaration"],
    count: KeywordHelpers::zig,
};

const ZIG_TYPES: &[&str] = &[
    "struct_declaration",
    "enum_declaration",
    "union_declaration",
    "opaque_declaration",
];

struct KeywordHelpers;

impl KeywordHelpers {
    fn rust(scope: &Scope, node: Node) -> usize {
        let public = VisibilityHelpers::modifiers(scope, node).contains(&"pub");
        usize::from(public)
    }

    fn java(scope: &Scope, node: Node) -> usize {
        let in_interface = node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "interface_body" | "annotation_type_body"));
        Self::public_or_implicit(scope, node, in_interface)
    }

    fn csharp(scope: &Scope, node: Node) -> usize {
        let in_interface = node
            .parent()
            .and_then(|list| list.parent())
            .is_some_and(|p| p.kind() == "interface_declaration");
        Self::public_or_implicit(scope, node, in_interface)
    }

    fn public_or_implicit(scope: &Scope, node: Node, implicit: bool) -> usize {
        let public = VisibilityHelpers::has_any(scope, node, &["public"])
            || (implicit && !VisibilityHelpers::has_any(scope, node, &["private"]));
        usize::from(public)
    }

    fn not_hidden(scope: &Scope, node: Node) -> usize {
        usize::from(!VisibilityHelpers::has_any(scope, node, HIDDEN))
    }

    fn swift(scope: &Scope, node: Node) -> usize {
        usize::from(VisibilityHelpers::has_any(scope, node, &["public", "open"]))
    }

    fn zig(scope: &Scope, node: Node) -> usize {
        if !VisibilityHelpers::modifiers(scope, node).contains(&"pub") {
            return 0;
        }
        if node.kind() == "function_declaration" {
            return 1;
        }
        let mut cursor = node.walk();
        let is_type = node
            .children(&mut cursor)
            .any(|child| ZIG_TYPES.contains(&child.kind()));
        usize::from(is_type)
    }
}
