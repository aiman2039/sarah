mod classes;
mod other;

use std::collections::BTreeMap;

use tree_sitter::Node;

use self::classes::ClassHelpers;
use self::other::OtherTypes;
use super::lang::Lang;
use super::scope::Scope;
use crate::report::WarningKind;
use crate::units::{Fields, LineNo, Methods};

#[derive(Debug, Clone)]
pub struct TypeStats {
    pub name: String,
    pub kind: &'static str,
    pub start_line: LineNo,
    pub end_line: LineNo,
    pub fields: Fields,
    pub methods: Methods,
    pub allow: Vec<WarningKind>,
}

struct NewType<'a> {
    pub name: String,
    pub kind: &'static str,
    pub node: Node<'a>,
    pub fields: usize,
    pub methods: usize,
}

impl TypeStats {
    fn from_node(input: NewType<'_>) -> Self {
        let NewType {
            name,
            kind,
            node,
            fields,
            methods,
        } = input;
        Self {
            name,
            kind,
            start_line: LineNo(node.start_position().row.saturating_add(1)),
            end_line: LineNo(node.end_position().row.saturating_add(1)),
            fields: Fields(fields),
            methods: Methods(methods),
            allow: Vec::new(),
        }
    }
}

/// Dispatches type extraction per language family.
pub(super) struct TypeHelpers;

impl TypeHelpers {
    pub(super) fn extract(scope: &Scope) -> Vec<TypeStats> {
        match scope.lang {
            Lang::Rust => StructHelpers::rust_types(scope),
            Lang::Go => StructHelpers::go_types(scope),
            Lang::Zig => OtherTypes::zig_types(scope),
            Lang::Elixir => OtherTypes::elixir_types(scope),
            Lang::Bash | Lang::Lua => Vec::new(),
            _ => ClassHelpers::extract(scope),
        }
    }
}

/// Rust and Go types, where methods live in `impl` or receiver declarations outside `struct`.
struct StructHelpers;

impl StructHelpers {
    fn rust_types(scope: &Scope) -> Vec<TypeStats> {
        let mut by_name: BTreeMap<String, TypeStats> = BTreeMap::new();
        let mut cursor = scope.node.walk();
        for node in scope.node.named_children(&mut cursor) {
            if node.kind() == "struct_item" {
                if let Some(stats) = Self::rust_struct(scope, node) {
                    by_name.insert(stats.name.clone(), stats);
                }
            } else if node.kind() == "impl_item" && node.child_by_field_name("trait").is_none() {
                Self::add_rust_impl(scope, node, &mut by_name);
            }
        }
        Self::sorted(by_name)
    }

    fn rust_struct(scope: &Scope, node: Node) -> Option<TypeStats> {
        let name = scope.text(node.child_by_field_name("name")?).trim();
        if name.is_empty() {
            return None;
        }
        let fields = node.child_by_field_name("body").map_or(0, |body| {
            let mut cursor = body.walk();
            body.named_children(&mut cursor)
                .filter(|c| c.kind() == "field_declaration" || body.kind().starts_with("ordered_"))
                .count()
        });
        Some(TypeStats::from_node(NewType {
            name: name.to_string(),
            kind: "struct",
            node,
            fields,
            methods: 0,
        }))
    }

    fn add_rust_impl(scope: &Scope, node: Node, by_name: &mut BTreeMap<String, TypeStats>) {
        let Some(ty) = node.child_by_field_name("type") else {
            return;
        };
        let raw = scope.text(ty).trim();
        let name = raw.split('<').next().unwrap_or(raw).trim();
        if name.is_empty() {
            return;
        }
        let methods = node.child_by_field_name("body").map_or(0, |body| {
            let mut cursor = body.walk();
            body.named_children(&mut cursor)
                .filter(|c| c.kind() == "function_item")
                .count()
        });
        Self::record_methods(by_name, name, (node, methods));
    }

    fn go_types(scope: &Scope) -> Vec<TypeStats> {
        let mut by_name: BTreeMap<String, TypeStats> = BTreeMap::new();
        let mut stack = vec![scope.node];
        while let Some(node) = stack.pop() {
            if node.kind() == "type_spec" {
                if let Some(stats) = Self::go_struct(scope, node) {
                    by_name.insert(stats.name.clone(), stats);
                }
            } else if node.kind() == "method_declaration" {
                Self::add_go_method(scope, node, &mut by_name);
            } else {
                let mut cursor = node.walk();
                stack.extend(node.named_children(&mut cursor));
            }
        }
        Self::sorted(by_name)
    }

    fn go_struct(scope: &Scope, node: Node) -> Option<TypeStats> {
        let ty = node.child_by_field_name("type")?;
        if ty.kind() != "struct_type" {
            return None;
        }
        let name = scope.text(node.child_by_field_name("name")?).trim();
        let mut cursor = ty.walk();
        let fields = ty
            .named_children(&mut cursor)
            .find(|c| c.kind() == "field_declaration_list")
            .map_or(0, |l| {
                let mut fc = l.walk();
                l.named_children(&mut fc)
                    .filter(|c| c.kind() == "field_declaration")
                    .map(|f| {
                        let mut nc = f.walk();
                        f.children_by_field_name("name", &mut nc).count().max(1)
                    })
                    .fold(0usize, usize::saturating_add)
            });
        Some(TypeStats::from_node(NewType {
            name: name.to_string(),
            kind: "struct",
            node,
            fields,
            methods: 0,
        }))
    }

    fn add_go_method(scope: &Scope, node: Node, by_name: &mut BTreeMap<String, TypeStats>) {
        let Some(recv) = node.child_by_field_name("receiver") else {
            return;
        };
        let Some(last) = scope
            .text(recv)
            .trim_matches(['(', ')'])
            .split_whitespace()
            .last()
        else {
            return;
        };
        let name = last
            .trim_start_matches('*')
            .split('[')
            .next()
            .unwrap_or(last);
        if !name.is_empty() {
            Self::record_methods(by_name, name, (node, 1));
        }
    }

    fn record_methods(
        by_name: &mut BTreeMap<String, TypeStats>,
        name: &str,
        span_and_count: (Node, usize),
    ) {
        let (node, count) = span_and_count;
        let entry = by_name.entry(name.to_string()).or_insert_with(|| {
            TypeStats::from_node(NewType {
                name: name.to_string(),
                kind: "struct",
                node,
                fields: 0,
                methods: 0,
            })
        });
        entry.methods = Methods(entry.methods.0.saturating_add(count));
    }

    fn sorted(by_name: BTreeMap<String, TypeStats>) -> Vec<TypeStats> {
        let mut out: Vec<_> = by_name.into_values().collect();
        out.sort_by_key(|t| t.start_line);
        out
    }
}
