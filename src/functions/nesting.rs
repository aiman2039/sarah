use tree_sitter::Node;

use super::scope::Scope;
use crate::units::{Depth, LineNo, LineSpan, Lines};

/// Control-flow nesting summary for one function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct NestingStats {
    pub depth: Depth,
    /// Span of the deepest nested control-flow block, when `depth >= 1`.
    pub deepest: Option<LineSpan>,
    /// Largest top-level control-flow block inside the function (>= 5 lines).
    pub largest: Option<LineSpan>,
}

struct NestingAcc {
    stats: NestingStats,
    fn_lines: Lines,
}

/// Stateless helpers that measure control-flow nesting in a syntax tree.
pub(super) struct NestingHelpers;

impl NestingHelpers {
    /// Deepest nesting and split-candidate block spans below the function node.
    pub(super) fn measure(scope: &Scope) -> NestingStats {
        let kinds = scope.grammar().nesting;
        let mut acc = NestingAcc {
            stats: NestingStats::default(),
            fn_lines: Self::span_of(scope.node).lines(),
        };
        let mut stack = vec![(scope.node, 0usize)];
        while let Some((parent, depth)) = stack.pop() {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                let opens =
                    kinds.contains(&scope.kind_of(child)) && !Scope::is_else_if(child, parent);
                let next = depth.saturating_add(usize::from(opens));
                if opens {
                    acc.record(Self::span_of(child), next);
                }
                stack.push((child, next));
            }
        }
        acc.stats
    }

    fn span_of(node: Node) -> LineSpan {
        LineSpan {
            start: LineNo(node.start_position().row.saturating_add(1)),
            end: LineNo(node.end_position().row.saturating_add(1)),
        }
    }
}

impl NestingAcc {
    fn record(&mut self, span: LineSpan, depth: usize) {
        if depth > self.stats.depth.0
            || (depth == self.stats.depth.0
                && self
                    .stats
                    .deepest
                    .is_none_or(|prev| span.lines() > prev.lines()))
        {
            self.stats.depth = Depth(depth);
            self.stats.deepest = Some(span);
        }
        let lines = span.lines();
        if depth == 1
            && lines >= Lines(5)
            && lines < self.fn_lines
            && self.stats.largest.is_none_or(|prev| lines > prev.lines())
        {
            self.stats.largest = Some(span);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::count::Tokenizer;
    use crate::functions::FnStats;
    use crate::units::Depth;

    fn depths(path: &str, src: &str) -> Vec<(String, Depth)> {
        FnStats::extract(Path::new(path), src, Tokenizer::O200k)
            .unwrap()
            .into_iter()
            .map(|f| (f.name, f.depth))
            .collect()
    }

    #[test]
    fn counts_nesting_and_flattens_else_if() {
        let src = "fn flat() { 1; }\n\
                   fn nested(x: u8) {\n  for _ in 0..1 {\n    if x > 0 {\n      match x { _ => {} }\n    }\n  }\n}\n\
                   fn chain(x: u8) {\n  if x == 0 {} else if x == 1 {} else if x == 2 {}\n}\n";
        assert_eq!(
            depths("x.rs", src),
            vec![
                ("flat".into(), Depth(0)),
                ("nested".into(), Depth(3)),
                ("chain".into(), Depth(1)),
            ]
        );
    }

    #[test]
    fn go_else_if_chain() {
        let src =
            "package p\nfunc f(x int) {\n\tif x == 0 {\n\t} else if x == 1 {\n\t\tfor {}\n\t}\n}\n";
        assert_eq!(depths("x.go", src), vec![("f".into(), Depth(2))]);
    }
}
