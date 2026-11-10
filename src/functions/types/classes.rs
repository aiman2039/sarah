use std::collections::HashSet;

use tree_sitter::Node;

use super::{NewType, TypeStats};
use crate::functions::lang::Lang;
use crate::functions::scope::Scope;

const CLASS_KINDS: &[&str] = &[
    "class_declaration",
    "class_definition",
    "class_specifier",
    "class",
    "struct_declaration",
    "struct_specifier",
    "record_declaration",
];
const FIELD_KINDS: &[&str] = &[
    "field_declaration",
    "field_definition",
    "public_field_definition",
    "property_declaration",
    "val_definition",
    "var_definition",
    "container_field",
];
const METHOD_KINDS: &[&str] = &[
    "method_declaration",
    "method_definition",
    "function_declaration",
    "function_definition",
    "constructor_declaration",
    "compact_constructor_declaration",
    "init_declaration",
    "method",
    "singleton_method",
];

/// Languages where fields and methods live inside the class or struct body.
pub(super) struct ClassHelpers;

impl ClassHelpers {
    pub(super) fn extract(scope: &Scope) -> Vec<TypeStats> {
        let mut out = Vec::new();
        let mut stack = vec![scope.node];
        while let Some(node) = stack.pop() {
            if CLASS_KINDS.contains(&node.kind())
                && let Some(stats) = Self::class_node(scope, node)
            {
                out.push(stats);
            }
            let mut cursor = node.walk();
            let children: Vec<_> = node.named_children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }
        out
    }

    fn class_node(scope: &Scope, node: Node) -> Option<TypeStats> {
        let body = Self::body_of(node)?;
        let name = Self::type_name(scope, node)?;
        let kind = if node.kind().starts_with("struct") {
            "struct"
        } else {
            "class"
        };
        let (fields, methods) = if scope.lang == Lang::Python {
            Self::python_members(scope, body)
        } else {
            Self::standard_members(scope, node, body)
        };
        Some(TypeStats::from_node(NewType {
            name,
            kind,
            node,
            fields,
            methods,
        }))
    }

    fn body_of(node: Node) -> Option<Node> {
        if let Some(body) = node.child_by_field_name("body") {
            return Some(body);
        }
        let mut cursor = node.walk();
        node.named_children(&mut cursor).find(|c| {
            matches!(
                c.kind(),
                "class_body" | "declaration_list" | "field_declaration_list" | "template_body"
            )
        })
    }

    fn type_name(scope: &Scope, node: Node) -> Option<String> {
        if let Some(n) = node.child_by_field_name("name") {
            let text = scope.text(n).trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .find(|c| c.kind().contains("identifier") || c.kind() == "constant")
            .map(|c| scope.text(c).trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn standard_members(scope: &Scope, node: Node, body: Node) -> (usize, usize) {
        let mut fields = Self::ctor_fields(scope, node);
        let mut methods = 0usize;
        let mut cursor = body.walk();
        for member in body.named_children(&mut cursor) {
            if METHOD_KINDS.contains(&member.kind()) {
                methods = methods.saturating_add(1);
            } else if FIELD_KINDS.contains(&member.kind()) {
                fields = fields.saturating_add(1);
            }
        }
        (fields, methods)
    }

    fn ctor_fields(scope: &Scope, node: Node) -> usize {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .filter(|c| matches!(c.kind(), "primary_constructor" | "class_parameters"))
            .flat_map(|ctor| {
                let mut cc = ctor.walk();
                ctor.named_children(&mut cc)
                    .filter(|p| {
                        let t = scope.text(*p);
                        t.contains("val ") || t.contains("var ")
                    })
                    .collect::<Vec<_>>()
            })
            .count()
    }

    fn python_members(scope: &Scope, body: Node) -> (usize, usize) {
        let mut attrs: HashSet<String> = HashSet::new();
        let mut methods = 0usize;
        let mut cursor = body.walk();
        for stmt in body.named_children(&mut cursor) {
            let inner = if stmt.kind() == "decorated_definition" {
                stmt.child_by_field_name("definition").unwrap_or(stmt)
            } else {
                stmt
            };
            if inner.kind() == "function_definition" {
                methods = methods.saturating_add(1);
                Self::python_self_attrs(scope, inner, &mut attrs);
            } else if inner.kind() == "expression_statement" {
                Self::python_class_attr(scope, inner, &mut attrs);
            }
        }
        (attrs.len(), methods)
    }

    fn python_class_attr(scope: &Scope, stmt: Node, attrs: &mut HashSet<String>) {
        let mut cursor = stmt.walk();
        for child in stmt.named_children(&mut cursor) {
            if child.kind() == "assignment"
                && let Some(left) = child.child_by_field_name("left")
                && left.kind() == "identifier"
            {
                attrs.insert(scope.text(left).to_string());
            }
        }
    }

    fn python_self_attrs(scope: &Scope, func: Node, attrs: &mut HashSet<String>) {
        let mut stack = vec![func];
        while let Some(node) = stack.pop() {
            if node.kind() == "assignment"
                && let Some(left) = node.child_by_field_name("left")
                && let Some(attr) = scope.text(left).strip_prefix("self.")
                && !attr.contains('.')
            {
                attrs.insert(attr.trim().to_string());
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
    }
}
