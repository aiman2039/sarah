use tree_sitter::Node;

use crate::lang::Lang;
use crate::lang::Scope;
use crate::units::Returns;

/// Which nodes are explicit returns, and which nodes start a nested function.
struct ReturnSpec {
    /// Kinds of a `return` statement or expression.
    returns: &'static [&'static str],
    /// The node's first child must read as this word (Swift's `control_transfer_statement`
    /// also covers `break`; Bash's `return` is a plain command).
    keyword: Option<&'static str>,
    /// Functions, closures and lambdas: their returns belong to them, not to the outer function.
    nested: &'static [&'static str],
}

const RETURN_STATEMENT: &[&str] = &["return_statement"];
const RETURN_EXPRESSION: &[&str] = &["return_expression"];

const RUST: ReturnSpec = ReturnSpec {
    returns: RETURN_EXPRESSION,
    keyword: None,
    nested: &["function_item", "closure_expression"],
};
const PYTHON: ReturnSpec = ReturnSpec {
    returns: RETURN_STATEMENT,
    keyword: None,
    nested: &["function_definition", "lambda"],
};
const JS: ReturnSpec = ReturnSpec {
    returns: RETURN_STATEMENT,
    keyword: None,
    nested: &[
        "function_declaration",
        "function_expression",
        "arrow_function",
        "generator_function",
        "generator_function_declaration",
        "method_definition",
    ],
};
const GO: ReturnSpec = ReturnSpec {
    returns: RETURN_STATEMENT,
    keyword: None,
    nested: &["function_declaration", "method_declaration", "func_literal"],
};
const JAVA: ReturnSpec = ReturnSpec {
    returns: RETURN_STATEMENT,
    keyword: None,
    nested: &[
        "method_declaration",
        "constructor_declaration",
        "lambda_expression",
    ],
};
const C: ReturnSpec = ReturnSpec {
    returns: RETURN_STATEMENT,
    keyword: None,
    nested: &["function_definition"],
};
const CPP: ReturnSpec = ReturnSpec {
    returns: &["return_statement", "co_return_statement"],
    keyword: None,
    nested: &["function_definition", "lambda_expression"],
};
const CSHARP: ReturnSpec = ReturnSpec {
    returns: RETURN_STATEMENT,
    keyword: None,
    nested: &[
        "method_declaration",
        "constructor_declaration",
        "local_function_statement",
        "lambda_expression",
        "anonymous_method_expression",
    ],
};
const RUBY: ReturnSpec = ReturnSpec {
    returns: &["return"],
    keyword: None,
    nested: &["method", "singleton_method", "lambda"],
};
const PHP: ReturnSpec = ReturnSpec {
    returns: RETURN_STATEMENT,
    keyword: None,
    nested: &[
        "function_definition",
        "method_declaration",
        "anonymous_function",
        "arrow_function",
    ],
};
const KOTLIN: ReturnSpec = ReturnSpec {
    returns: RETURN_EXPRESSION,
    keyword: None,
    nested: &[
        "function_declaration",
        "lambda_literal",
        "anonymous_function",
    ],
};
const SWIFT: ReturnSpec = ReturnSpec {
    returns: &["control_transfer_statement"],
    keyword: Some("return"),
    nested: &["function_declaration", "lambda_literal"],
};
const SCALA: ReturnSpec = ReturnSpec {
    returns: RETURN_EXPRESSION,
    keyword: None,
    nested: &["function_definition", "lambda_expression"],
};
const LUA: ReturnSpec = ReturnSpec {
    returns: RETURN_STATEMENT,
    keyword: None,
    nested: &["function_declaration", "function_definition"],
};
/// Elixir has no `return`; every function counts 0.
const ELIXIR: ReturnSpec = ReturnSpec {
    returns: &[],
    keyword: None,
    nested: &[],
};
const ZIG: ReturnSpec = ReturnSpec {
    returns: RETURN_EXPRESSION,
    keyword: None,
    nested: &["function_declaration"],
};
const BASH: ReturnSpec = ReturnSpec {
    returns: &["command"],
    keyword: Some("return"),
    nested: &["function_definition"],
};

/// Stateless helpers that count explicit `return`s in one function.
pub(super) struct ReturnHelpers;

impl ReturnHelpers {
    /// Explicit returns below the function node. A tag may cover a wrapper (JS
    /// `const f = () => ...`), so the first function node on the way down is the
    /// function itself; any function node below it is nested and skipped.
    pub(super) fn count(scope: &Scope) -> Returns {
        let spec = Self::spec(scope.lang);
        let mut total = 0usize;
        let inside = spec.nested.contains(&scope.node.kind());
        let mut stack = vec![(scope.node, inside)];
        while let Some((parent, inside)) = stack.pop() {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                let is_fn = spec.nested.contains(&child.kind());
                if is_fn && inside {
                    continue;
                }
                if Self::is_return(scope, spec, child) {
                    total = total.saturating_add(1);
                }
                stack.push((child, inside || is_fn));
            }
        }
        Returns(total)
    }

    fn is_return(scope: &Scope, spec: &ReturnSpec, node: Node) -> bool {
        if !spec.returns.contains(&node.kind()) {
            return false;
        }
        match spec.keyword {
            None => true,
            Some(word) => node.child(0).is_some_and(|first| scope.text(first) == word),
        }
    }

    fn spec(lang: Lang) -> &'static ReturnSpec {
        match lang {
            Lang::Rust => &RUST,
            Lang::Python => &PYTHON,
            Lang::JavaScript | Lang::TypeScript | Lang::Tsx => &JS,
            Lang::Go => &GO,
            Lang::Java => &JAVA,
            Lang::C => &C,
            Lang::Cpp => &CPP,
            Lang::CSharp => &CSHARP,
            Lang::Ruby => &RUBY,
            Lang::Php => &PHP,
            Lang::Kotlin => &KOTLIN,
            Lang::Swift => &SWIFT,
            Lang::Scala => &SCALA,
            Lang::Lua => &LUA,
            Lang::Elixir => &ELIXIR,
            Lang::Zig => &ZIG,
            Lang::Bash => &BASH,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::count::Tokenizer;
    use crate::functions::FnStats;

    fn returns(path: &str, src: &str) -> Vec<(String, usize)> {
        FnStats::extract(Path::new(path), src, Tokenizer::O200k)
            .unwrap()
            .into_iter()
            .map(|f| (f.name, f.returns.0))
            .collect()
    }

    fn of(path: &str, src: &str, name: &str) -> usize {
        let all = returns(path, src);
        all.iter()
            .find(|(n, _)| n == name)
            .map_or_else(|| panic!("{path}: no {name} in {all:?}"), |(_, r)| *r)
    }

    #[test]
    fn kinds_exist() {
        for lang in Lang::ALL {
            let language = lang.language();
            let spec = ReturnHelpers::spec(lang);
            for kind in spec.returns.iter().chain(spec.nested) {
                assert_ne!(language.id_for_node_kind(kind, true), 0, "{lang:?} {kind}");
            }
        }
    }

    #[test]
    fn rust_skips_closures_tail_and_question_mark() {
        let src = "fn f(x: u8) -> Option<u8> {\n    if x == 0 { return None; }\n    let g = |y: u8| { return y; };\n    fn inner() -> u8 { return 1; }\n    let _ = Some(1)?;\n    if x == 1 { return Some(1); }\n    Some(g(x))\n}\n";
        assert_eq!(of("x.rs", src, "f"), 2);
        assert_eq!(of("x.rs", src, "inner"), 1);
    }

    #[test]
    fn python_typescript_go() {
        let py = "def f(x):\n    if x:\n        return 1\n    g = lambda: 2\n    def h():\n        return 3\n    return 4\n";
        assert_eq!(of("x.py", py, "f"), 2);
        let ts = "const f = (x: number) => {\n  if (x) { return 1; }\n  const g = () => { return 2; };\n  return 3;\n};\nfunction k() {\n  [1].map(function () { return 1; });\n  return 0;\n}\n";
        assert_eq!(of("x.ts", ts, "f"), 2);
        assert_eq!(of("x.ts", ts, "k"), 1);
        let go = "package p\nfunc f(x int) int {\n\tg := func() int { return 1 }\n\tif x > 0 {\n\t\treturn g()\n\t}\n\treturn 0\n}\n";
        assert_eq!(of("x.go", go, "f"), 2);
    }

    #[test]
    fn keyword_returns() {
        let swift = "func f(x: Int) -> Int {\n  for _ in 0..<1 { break }\n  if x > 0 { return 1 }\n  return 0\n}\n";
        assert_eq!(of("x.swift", swift, "f"), 2);
        let bash = "f() {\n  echo hi\n  return 1\n}\n";
        assert_eq!(of("x.sh", bash, "f"), 1);
        let kt = "fun f(x: Int): Int {\n  if (x > 0) return 1\n  return 0\n}\n";
        assert_eq!(of("x.kt", kt, "f"), 2);
    }
}
