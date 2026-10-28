use super::scope::Scope;
use crate::units::Depth;

/// Stateless helpers that measure control-flow nesting in a syntax tree.
pub(super) struct NestingHelpers;

impl NestingHelpers {
    /// Deepest nesting below the function node.
    pub(super) fn depth(scope: &Scope) -> Depth {
        let kinds = scope.grammar().nesting;
        let mut max = 0;
        let mut stack = vec![(scope.node, 0usize)];
        while let Some((parent, depth)) = stack.pop() {
            max = max.max(depth);
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                let opens =
                    kinds.contains(&scope.kind_of(child)) && !Scope::is_else_if(child, parent);
                stack.push((child, depth.saturating_add(usize::from(opens))));
            }
        }
        Depth(max)
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
