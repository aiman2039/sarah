use tree_sitter::Node;

use super::scope::Scope;
use crate::units::{Cognitive, Cyclomatic};

const LOGICAL_OPERATORS: &[&str] = &["&&", "||", "and", "or"];
/// How far up an `else` looks for the statement it belongs to.
const ELSE_OWNER_DEPTH: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Complexity {
    pub cyclomatic: Cyclomatic,
    pub cognitive: Cognitive,
}

/// What one node adds to the complexities, and whether it nests its children.
#[derive(Default)]
struct Step {
    cyclomatic: usize,
    cognitive: usize,
    nests: bool,
}

/// Stateless helpers that score a function's control flow.
pub(super) struct ComplexityHelpers;

impl ComplexityHelpers {
    pub(super) fn measure(scope: &Scope) -> Complexity {
        let mut cyclomatic = 1usize;
        let mut cognitive = 0usize;
        let mut stack = vec![(scope.node, 0usize)];
        while let Some((parent, nesting)) = stack.pop() {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                let step = Self::step(scope, child, nesting);
                cyclomatic = cyclomatic.saturating_add(step.cyclomatic);
                cognitive = cognitive.saturating_add(step.cognitive);
                stack.push((child, nesting.saturating_add(usize::from(step.nests))));
            }
        }
        Complexity {
            cyclomatic: Cyclomatic(cyclomatic),
            cognitive: Cognitive(cognitive),
        }
    }

    fn step(scope: &Scope, node: Node, nesting: usize) -> Step {
        let g = scope.grammar();
        let kind = scope.kind_of(node);
        if g.branches.contains(&kind) || g.switches.contains(&kind) {
            let chained = node.parent().is_some_and(|p| Scope::is_else_if(node, p));
            return Step {
                cyclomatic: usize::from(g.branches.contains(&kind)),
                cognitive: if chained {
                    1
                } else {
                    nesting.saturating_add(1)
                },
                nests: !chained,
            };
        }
        if g.elifs.contains(&kind) {
            return Step {
                cyclomatic: 1,
                cognitive: 1,
                nests: false,
            };
        }
        if g.cases.contains(&kind) {
            return Step {
                cyclomatic: 1,
                ..Step::default()
            };
        }
        if let Some(op) = Self::logical_operator(scope, node) {
            return Step {
                cyclomatic: 1,
                cognitive: usize::from(!Self::continues_run(scope, node, op)),
                nests: false,
            };
        }
        Step {
            cognitive: usize::from(Self::is_plain_else(scope, node)),
            ..Step::default()
        }
    }

    /// The operator of a logical binary node (`&&`, `||`, `and`, `or`).
    fn logical_operator<'a>(scope: &Scope<'a>, node: Node) -> Option<&'a str> {
        if !scope.grammar().binaries.contains(&node.kind()) {
            return None;
        }
        let op = node
            .child_by_field_name("operator")
            .or_else(|| node.child_by_field_name("op"))
            .or_else(|| Self::first_anonymous_child(node))?;
        let text = scope.text(op);
        LOGICAL_OPERATORS.contains(&text).then_some(text)
    }

    fn first_anonymous_child(node: Node) -> Option<Node> {
        let mut cursor = node.walk();
        node.children(&mut cursor).find(|c| !c.is_named())
    }

    /// `a && b && c` is one run (+1); the inner `&&` continues its parent's run.
    fn continues_run(scope: &Scope, node: Node, op: &str) -> bool {
        node.parent()
            .and_then(|p| Self::logical_operator(scope, p))
            .is_some_and(|parent_op| parent_op == op)
    }

    /// An `else` keyword of an `if` that is not followed by another `if` (that one counts itself).
    fn is_plain_else(scope: &Scope, node: Node) -> bool {
        if node.kind() != "else" || node.child_count() != 0 {
            return false;
        }
        let chains = node
            .next_named_sibling()
            .is_some_and(|next| next.kind().contains("if"));
        !chains && Self::else_owner_is_if(scope, node)
    }

    /// The nearest enclosing control-flow node is an `if` (not a `switch`, loop or `guard`).
    fn else_owner_is_if(scope: &Scope, node: Node) -> bool {
        let g = scope.grammar();
        let tables = [g.branches, g.elifs, g.switches, g.cases];
        std::iter::successors(node.parent(), Node::parent)
            .take(ELSE_OWNER_DEPTH)
            .map(|n| scope.kind_of(n))
            .find(|kind| tables.iter().any(|t| t.contains(kind)))
            .is_some_and(|kind| kind.contains("if") || kind == "unless")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::count::Tokenizer;
    use crate::functions::FnStats;

    /// (name, cyclomatic, cognitive) of every function in `src`.
    fn scores(path: &str, src: &str) -> Vec<(String, usize, usize)> {
        FnStats::extract(Path::new(path), src, Tokenizer::O200k)
            .unwrap()
            .into_iter()
            .map(|f| (f.name, f.cyclomatic.0, f.cognitive.0))
            .collect()
    }

    #[test]
    fn rust_nesting_else_chains_and_boolean_runs() {
        let src = "fn f(a: u8, b: u8) -> u8 {\n\
                   \x20   if a > 0 && b > 0 && a < 9 {\n\
                   \x20       for _ in 0..1 {\n\
                   \x20           if b == 1 || a == 2 { return 1; }\n\
                   \x20       }\n\
                   \x20   } else if a == 3 {\n\
                   \x20       return 2;\n\
                   \x20   } else {\n\
                   \x20       match a { 1 => {}, _ => {} }\n\
                   \x20   }\n\
                   \x20   0\n\
                   }\n\
                   fn flat() {}\n";
        // Cognitive: if 1, && run 1, for 2, nested if 3, || 1, else if 1, else 1, match 2.
        // Cyclomatic: 1 + if, 2 &&, for, if, ||, else if, 2 arms.
        assert_eq!(
            scores("x.rs", src),
            vec![("f".into(), 10, 12), ("flat".into(), 1, 0)]
        );
    }

    #[test]
    fn mixed_operators_are_separate_runs() {
        let src = "function f(a, b, c) {\n  return a && b || c;\n}\n";
        assert_eq!(scores("x.js", src), vec![("f".into(), 3, 2)]);
    }

    #[test]
    fn elif_and_case_arms() {
        let py = "def g(a, b):\n    if a and b:\n        pass\n    elif a:\n        pass\n    else:\n        pass\n";
        assert_eq!(scores("x.py", py), vec![("g".into(), 4, 4)]);
        let go = "package p\nfunc h(a int) {\n\tswitch a {\n\tcase 1:\n\tcase 2:\n\t}\n}\n";
        assert_eq!(scores("x.go", go), vec![("h".into(), 3, 1)]);
    }

    #[test]
    fn else_of_a_switch_or_guard_is_not_counted() {
        let kt = "fun k(a: Int): Int {\n    return when (a) {\n        1 -> 1\n        else -> 2\n    }\n}\n";
        assert_eq!(scores("x.kt", kt), vec![("k".into(), 3, 1)]);
        let swift = "func s(a: Int) -> Int {\n    guard a > 0 else { return 0 }\n    return 1\n}\n";
        assert_eq!(scores("x.swift", swift), vec![("s".into(), 2, 1)]);
    }

    #[test]
    fn elixir_keyword_calls() {
        let src = "defmodule M do\n  def ex(a, b) do\n    if a && b do\n      case a do\n        1 -> :one\n        _ -> :other\n      end\n    else\n      :no\n    end\n  end\nend\n";
        assert_eq!(scores("x.ex", src), vec![("ex".into(), 3, 5)]);
    }
}
