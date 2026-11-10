use tree_sitter::Node;

use super::{NewType, TypeStats};
use crate::functions::scope::Scope;

/// Elixir (`defmodule`) and Zig (`const T = struct { ... }`) types.
pub(super) struct OtherTypes;

impl OtherTypes {
    pub(super) fn zig_types(scope: &Scope) -> Vec<TypeStats> {
        let mut out = Vec::new();
        let mut cursor = scope.node.walk();
        for node in scope.node.named_children(&mut cursor) {
            if node.kind() == "variable_declaration"
                && let Some(stats) = Self::zig_decl(scope, node)
            {
                out.push(stats);
            }
        }
        out
    }

    fn zig_decl(scope: &Scope, node: Node) -> Option<TypeStats> {
        let mut cursor = node.walk();
        let children: Vec<_> = node.named_children(&mut cursor).collect();
        let name_node = children.iter().find(|c| c.kind() == "identifier")?;
        let container = children.iter().find(|c| c.kind() == "container_decl")?;
        let mut cc = container.walk();
        let mut fields = 0usize;
        let mut methods = 0usize;
        for item in container.named_children(&mut cc) {
            if item.kind() == "container_field" {
                fields = fields.saturating_add(1);
            } else if item.kind() == "function_declaration" {
                methods = methods.saturating_add(1);
            }
        }
        Some(TypeStats::from_node(NewType {
            name: scope.text(*name_node).trim().to_string(),
            kind: "struct",
            node,
            fields,
            methods,
        }))
    }

    pub(super) fn elixir_types(scope: &Scope) -> Vec<TypeStats> {
        let mut out = Vec::new();
        let mut stack = vec![scope.node];
        while let Some(node) = stack.pop() {
            if node.kind() == "call"
                && let Some(stats) = Self::elixir_module(scope, node)
            {
                out.push(stats);
            }
            let mut cursor = node.walk();
            let children: Vec<_> = node.named_children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }
        out
    }

    fn elixir_module(scope: &Scope, node: Node) -> Option<TypeStats> {
        let target = node.child_by_field_name("target")?;
        if scope.text(target) != "defmodule" {
            return None;
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.named_children(&mut cursor).collect();
        let args = children.iter().find(|c| c.kind() == "arguments")?;
        let mut ac = args.walk();
        let name_node = args.named_children(&mut ac).next()?;
        let do_block = children.iter().find(|c| c.kind() == "do_block")?;
        let (fields, methods) = Self::elixir_body(scope, *do_block);
        Some(TypeStats::from_node(NewType {
            name: scope.text(name_node).trim().to_string(),
            kind: "module",
            node,
            fields,
            methods,
        }))
    }

    fn elixir_body(scope: &Scope, do_block: Node) -> (usize, usize) {
        let mut fields = 0usize;
        let mut methods = 0usize;
        let mut cursor = do_block.walk();
        for item in do_block.named_children(&mut cursor) {
            let Some(target) = item.child_by_field_name("target") else {
                continue;
            };
            match scope.text(target) {
                "def" | "defp" => methods = methods.saturating_add(1),
                "defstruct" => fields = fields.saturating_add(Self::defstruct_fields(item)),
                _ => {}
            }
        }
        (fields, methods)
    }

    fn defstruct_fields(call: Node) -> usize {
        let mut cursor = call.walk();
        let Some(args) = call
            .named_children(&mut cursor)
            .find(|c| c.kind() == "arguments")
        else {
            return 0;
        };
        let mut ac = args.walk();
        let Some(first) = args.named_children(&mut ac).next() else {
            return 0;
        };
        let mut fc = first.walk();
        first.named_children(&mut fc).count().max(1)
    }
}
