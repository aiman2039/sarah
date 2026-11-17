//! Languages where names or `export` decide visibility: Go, Python, JavaScript/TypeScript.

use tree_sitter::Node;

use super::VisibilitySpec;
use crate::lang::Scope;

/// Capitalized functions, methods and types.
pub(super) const GO: VisibilitySpec = VisibilitySpec {
    items: &[
        "function_declaration",
        "method_declaration",
        "type_spec",
        "type_alias",
    ],
    opaque: &["function_declaration", "method_declaration"],
    count: ExportHelpers::go,
};

/// Module-level functions and classes whose name has no leading `_`.
pub(super) const PYTHON: VisibilitySpec = VisibilitySpec {
    items: &["function_definition", "class_definition"],
    opaque: &["function_definition", "class_definition"],
    count: ExportHelpers::python,
};

/// `export`ed functions, classes (plus their public methods), interfaces, types, enums,
/// function-valued `const`s and local `export { a, b }` names. `export` is only valid at
/// module or namespace level, so functions, classes and variable declarations are opaque.
pub(super) const JS: VisibilitySpec = VisibilitySpec {
    items: &["export_statement"],
    opaque: &[
        "export_statement",
        "function_declaration",
        "generator_function_declaration",
        "function_expression",
        "arrow_function",
        "generator_function",
        "method_definition",
        "class_declaration",
        "class",
        "lexical_declaration",
        "variable_declaration",
    ],
    count: ExportHelpers::js_export,
};

const JS_FUNCTION_VALUES: &[&str] = &[
    "arrow_function",
    "function_expression",
    "generator_function",
];
const JS_SINGLE: &[&str] = &[
    "function_declaration",
    "generator_function_declaration",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
];
const JS_CLASSES: &[&str] = &["class_declaration", "abstract_class_declaration", "class"];
const JS_METHODS: &[&str] = &["method_definition", "abstract_method_signature"];

struct ExportHelpers;

impl ExportHelpers {
    fn go(scope: &Scope, node: Node) -> usize {
        let exported = node
            .child_by_field_name("name")
            .and_then(|name| scope.text(name).chars().next())
            .is_some_and(char::is_uppercase);
        usize::from(exported)
    }

    fn python(scope: &Scope, node: Node) -> usize {
        let public = node
            .child_by_field_name("name")
            .is_some_and(|name| !scope.text(name).starts_with('_'));
        usize::from(public)
    }

    fn js_export(scope: &Scope, node: Node) -> usize {
        if node.child_by_field_name("source").is_some() {
            return 0;
        }
        let declared = node
            .child_by_field_name("declaration")
            .or_else(|| node.child_by_field_name("value"))
            .map_or(0, |decl| Self::js_declaration(scope, decl));
        let mut cursor = node.walk();
        let listed: usize = node
            .children(&mut cursor)
            .filter(|child| child.kind() == "export_clause")
            .map(|clause| clause.named_child_count())
            .sum();
        declared.saturating_add(listed)
    }

    fn js_declaration(scope: &Scope, decl: Node) -> usize {
        let kind = decl.kind();
        if JS_SINGLE.contains(&kind) || JS_FUNCTION_VALUES.contains(&kind) {
            return 1;
        }
        if JS_CLASSES.contains(&kind) {
            return Self::js_class(scope, decl);
        }
        if matches!(kind, "lexical_declaration" | "variable_declaration") {
            let mut cursor = decl.walk();
            return decl
                .named_children(&mut cursor)
                .filter_map(|d| d.child_by_field_name("value"))
                .map(|value| match value.kind() {
                    k if JS_CLASSES.contains(&k) => Self::js_class(scope, value),
                    k => usize::from(JS_FUNCTION_VALUES.contains(&k)),
                })
                .sum();
        }
        0
    }

    /// The class itself plus methods that are not `private`/`protected`/`#name`.
    fn js_class(scope: &Scope, class: Node) -> usize {
        let Some(body) = class.child_by_field_name("body") else {
            return 1;
        };
        let mut cursor = body.walk();
        let methods = body
            .named_children(&mut cursor)
            .filter(|m| JS_METHODS.contains(&m.kind()))
            .filter(|m| Self::js_public_member(scope, *m))
            .count();
        methods.saturating_add(1)
    }

    fn js_public_member(scope: &Scope, member: Node) -> bool {
        let private_name = member
            .child_by_field_name("name")
            .is_some_and(|name| name.kind() == "private_property_identifier");
        let mut cursor = member.walk();
        let hidden = member
            .children(&mut cursor)
            .any(|c| c.kind() == "accessibility_modifier" && scope.text(c) != "public");
        !private_name && !hidden
    }
}
