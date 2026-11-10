use tree_sitter::Node;

use super::lang::ParamSpec;
use super::scope::Scope;
use crate::units::Params;

/// Receivers and C's `(void)`, which declare no real parameter.
const NOT_PARAMS: &[&str] = &["self", "cls", "void"];
const BOOL_TYPES: &[&str] = &["bool", "boolean", "Bool", "Boolean", "_Bool"];
const BOOL_LITERALS: &[&str] = &["true", "false", "True", "False"];
const TYPE_NODE_KINDS: &[&str] = &[
    "user_type",
    "primitive_type",
    "boolean_type",
    "predefined_type",
    "type_annotation",
    "type_identifier",
];

pub(super) struct ParamCounts {
    pub total: Option<Params>,
    pub bools: Option<Params>,
}

/// Stateless helpers that count a function's parameters and boolean flags.
pub(super) struct ParamHelpers;

impl ParamHelpers {
    /// `None` when the language has no parameter table.
    pub(super) fn count(scope: &Scope) -> ParamCounts {
        let Some(spec) = scope.grammar().params.as_ref() else {
            return ParamCounts {
                total: None,
                bools: None,
            };
        };
        let list = Self::list(scope.node, spec).unwrap_or(scope.node);
        let mut total = 0usize;
        let mut bools = 0usize;
        let mut cursor = list.walk();
        for param in list.named_children(&mut cursor) {
            if !spec.items.contains(&param.kind()) || NOT_PARAMS.contains(&scope.text(param)) {
                continue;
            }
            let arity = if spec.names_per_item {
                Self::names_in(param).max(1)
            } else {
                1
            };
            total = total.saturating_add(arity);
            if Self::is_bool_param(scope, param) {
                bools = bools.saturating_add(arity);
            }
        }
        ParamCounts {
            total: Some(Params(total)),
            bools: Some(Params(bools)),
        }
    }

    fn is_bool_param(scope: &Scope, param: Node) -> bool {
        if param
            .child_by_field_name("type")
            .is_some_and(|ty| Self::is_bool_type(scope.text(ty)))
        {
            return true;
        }
        if param
            .child_by_field_name("value")
            .is_some_and(|val| Self::is_bool_literal(scope.text(val)))
        {
            return true;
        }
        let mut cursor = param.walk();
        param.named_children(&mut cursor).any(|child| {
            (TYPE_NODE_KINDS.contains(&child.kind()) && Self::is_bool_type(scope.text(child)))
                || Self::is_bool_literal(scope.text(child))
        })
    }

    fn is_bool_type(raw: &str) -> bool {
        let cleaned = raw
            .trim()
            .trim_start_matches(':')
            .trim()
            .trim_end_matches('?');
        BOOL_TYPES.contains(&cleaned)
    }

    fn is_bool_literal(raw: &str) -> bool {
        let cleaned = raw.trim().trim_start_matches('=').trim();
        BOOL_LITERALS.contains(&cleaned)
    }

    /// The `parameters` field, else the first parameter list outside the body
    /// (C's `declarator`, Kotlin's unnamed list).
    fn list<'a>(def: Node<'a>, spec: &ParamSpec) -> Option<Node<'a>> {
        let is_list = |n: &Node| spec.lists.contains(&n.kind());
        if let Some(field) = def.child_by_field_name("parameters").filter(is_list) {
            return Some(field);
        }
        let body = def.child_by_field_name("body");
        let mut stack = vec![def];
        while let Some(node) = stack.pop() {
            if is_list(&node) {
                return Some(node);
            }
            let mut cursor = node.walk();
            let children: Vec<_> = node
                .named_children(&mut cursor)
                .filter(|c| Some(*c) != body)
                .collect();
            stack.extend(children.into_iter().rev());
        }
        None
    }

    fn names_in(param: Node) -> usize {
        let mut cursor = param.walk();
        param.children_by_field_name("name", &mut cursor).count()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::count::Tokenizer;
    use crate::functions::FnStats;
    use crate::units::Params;

    fn params(path: &str, src: &str) -> Vec<(Option<Params>, Option<Params>)> {
        FnStats::extract(Path::new(path), src, Tokenizer::O200k)
            .unwrap()
            .into_iter()
            .map(|f| (f.params, f.bool_params))
            .collect()
    }

    #[test]
    fn counts_parameters_and_bools_without_receivers() {
        let cases = [
            (
                "x.rs",
                "struct S;\nimpl S {\n    fn m(&self, a: bool, b: u8, c: Vec<bool>) {}\n}\n",
                (3, 1),
            ),
            (
                "x.py",
                "class C:\n    def m(self, a: bool, b=False, *c, **d):\n        pass\n",
                (4, 2),
            ),
            (
                "x.go",
                "package p\nfunc (r R) h(a, b bool, c string) {}\n",
                (3, 2),
            ),
            ("x.c", "int f(void) {\n  return 1;\n}\n", (0, 0)),
            (
                "x.ts",
                "function f(a: boolean, b = true, c?: string) {}\n",
                (3, 2),
            ),
            ("x.kt", "fun k(a: Boolean, b: Int = 2): Int = b\n", (2, 1)),
        ];
        for (path, src, (want, bools)) in cases {
            assert_eq!(
                params(path, src),
                vec![(Some(Params(want)), Some(Params(bools)))],
                "{path}"
            );
        }
        assert_eq!(
            params("x.sh", "f() {\n  echo \"$1\"\n}\n"),
            vec![(None, None)]
        );
    }
}
