use tree_sitter::Node;

use super::lang::ParamSpec;
use super::scope::Scope;
use crate::units::Params;

/// Receivers and C's `(void)`, which declare no real parameter.
const NOT_PARAMS: &[&str] = &["self", "cls", "void"];

/// Stateless helpers that count a function's parameters.
pub(super) struct ParamHelpers;

impl ParamHelpers {
    /// `None` when the language has no parameter table.
    pub(super) fn count(scope: &Scope) -> Option<Params> {
        let spec = scope.grammar().params.as_ref()?;
        let list = Self::list(scope.node, spec).unwrap_or(scope.node);
        let mut cursor = list.walk();
        let count = list
            .named_children(&mut cursor)
            .filter(|p| spec.items.contains(&p.kind()) && !NOT_PARAMS.contains(&scope.text(*p)))
            .map(|p| {
                if spec.names_per_item {
                    Self::names_in(p).max(1)
                } else {
                    1
                }
            })
            .fold(0usize, usize::saturating_add);
        Some(Params(count))
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
            // Reversed so the stack visits children in source order.
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

    fn params(path: &str, src: &str) -> Vec<Option<Params>> {
        FnStats::extract(Path::new(path), src, Tokenizer::O200k)
            .unwrap()
            .into_iter()
            .map(|f| f.params)
            .collect()
    }

    #[test]
    fn counts_parameters_without_receivers() {
        let cases = [
            (
                "x.rs",
                "struct S;\nimpl S {\n    fn m(&self, a: u8, b: u8) {}\n}\n",
                2,
            ),
            (
                "x.py",
                "class C:\n    def m(self, a, b=1, *c, **d):\n        pass\n",
                4,
            ),
            (
                "x.go",
                "package p\nfunc (r R) h(a, b int, c string) {}\n",
                3,
            ),
            ("x.c", "int f(void) {\n  return 1;\n}\n", 0),
            ("x.ts", "function f(a: number, b?: string) {}\n", 2),
            (
                "x.swift",
                "func s(a: Int, b: Int) -> Int {\n    return a\n}\n",
                2,
            ),
            ("x.kt", "fun k(a: Int, b: Int = 2): Int = a\n", 2),
            ("x.lua", "function lu(a, b, ...)\nend\n", 3),
        ];
        for (path, src, want) in cases {
            assert_eq!(params(path, src), vec![Some(Params(want))], "{path}");
        }
        assert_eq!(params("x.sh", "f() {\n  echo \"$1\"\n}\n"), vec![None]);
    }
}
