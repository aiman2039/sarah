use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;
use tree_sitter::{Parser, Tree};
use tree_sitter_tags::TagsContext;

use super::complexity::{Complexity, ComplexityHelpers};
use super::imports::{ImportHelpers, ImportRef};
use super::lang::Lang;
use super::nesting::{NestingHelpers, NestingStats};
use super::params::{ParamCounts, ParamHelpers};
use super::scope::Scope;
use super::types::{TypeHelpers, TypeStats};
use crate::count::Tokenizer;
use crate::report::WarningKind;
use crate::units::{Cognitive, Cyclomatic, Depth, LineNo, LineSpan, Lines, Params, Tokens};

/// Everything extracted from one parsed syntax tree: functions, types and imports.
#[derive(Debug, Default)]
pub struct AstStats {
    pub functions: Vec<FnStats>,
    pub types: Vec<TypeStats>,
    pub imports: Vec<ImportRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FnStats {
    pub name: String,
    pub kind: String,
    pub start_line: LineNo,
    pub end_line: LineNo,
    pub lines: Lines,
    pub tokens: Tokens,
    /// Deepest control-flow nesting inside the function.
    pub depth: Depth,
    /// Declared parameters; `None` for languages without a parameter table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Params>,
    /// Declared boolean flag parameters; omitted when 0 or unsupported.
    #[serde(skip_serializing_if = "FnStats::skip_bool_params")]
    pub bool_params: Option<Params>,
    pub cyclomatic: Cyclomatic,
    pub cognitive: Cognitive,
    /// Warning kinds a `sarah-allow` comment above the function turns off.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<WarningKind>,
    #[serde(skip)]
    pub deepest_block: Option<LineSpan>,
    #[serde(skip)]
    pub largest_block: Option<LineSpan>,
}

/// Stateless AST helpers shared outside `functions`.
pub struct AstHelpers;

impl AstHelpers {
    pub fn is_code_file(path: &Path) -> bool {
        Lang::from_path(path).is_some()
    }
}

impl AstStats {
    /// Parse `source` once and extract functions, types and imports.
    pub fn extract(path: &Path, source: &str, tokenizer: Tokenizer) -> Option<Self> {
        let lang = Lang::from_path(path)?;
        let tree = FnStats::parse(lang, source)?;
        let root_scope = Scope {
            lang,
            node: tree.root_node(),
            source,
        };
        let functions = FnStats::from_tree(&root_scope, tokenizer)?;
        let types = TypeHelpers::extract(&root_scope);
        let imports = ImportHelpers::extract(&root_scope);
        Some(Self {
            functions,
            types,
            imports,
        })
    }
}

impl FnStats {
    /// Extract function/method definitions with their sizes. Returns `None` for unsupported languages.
    #[cfg(test)]
    pub fn extract(path: &Path, source: &str, tokenizer: Tokenizer) -> Option<Vec<Self>> {
        AstStats::extract(path, source, tokenizer).map(|ast| ast.functions)
    }

    pub fn skip_bool_params(params: &Option<Params>) -> bool {
        params.is_none_or(|p| p.0 == 0)
    }

    fn from_tree(root_scope: &Scope, tokenizer: Tokenizer) -> Option<Vec<Self>> {
        let lang = root_scope.lang;
        let source = root_scope.source;
        let config = lang.config()?;
        let mut ctx = TagsContext::new();
        let (tags, _) = ctx.generate_tags(config, source.as_bytes(), None).ok()?;

        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for tag in tags.filter_map(Result::ok) {
            let kind = config.syntax_type_name(tag.syntax_type_id);
            if !tag.is_definition || !matches!(kind, "function" | "method") {
                continue;
            }
            if !seen.insert(tag.range.clone()) {
                continue;
            }
            let (Some(body), Some(name), Some(before), Some(node)) = (
                source.get(tag.range.clone()),
                source.get(tag.name_range),
                source.get(..tag.range.start),
                root_scope
                    .node
                    .descendant_for_byte_range(tag.range.start, tag.range.end),
            ) else {
                continue;
            };
            let scope = Scope { lang, node, source };
            out.push(Self::measure_one(
                &scope,
                BodySlice {
                    name,
                    kind,
                    before,
                    body,
                },
                tokenizer,
            ));
        }
        Some(out)
    }

    fn measure_one(scope: &Scope, slice: BodySlice<'_>, tokenizer: Tokenizer) -> Self {
        let BodySlice {
            name,
            kind,
            before,
            body,
        } = slice;
        let Complexity {
            cyclomatic,
            cognitive,
        } = ComplexityHelpers::measure(scope);
        let NestingStats {
            depth,
            deepest,
            largest,
        } = NestingHelpers::measure(scope);
        let ParamCounts { total, bools } = ParamHelpers::count(scope);
        let start_line = LineNo(1).below(before.matches('\n').count());
        let end_line = start_line.below(body.matches('\n').count());
        Self {
            name: name.to_string(),
            kind: kind.to_string(),
            start_line,
            end_line,
            lines: start_line.span_to(end_line),
            tokens: tokenizer.count(body),
            depth,
            params: total,
            bool_params: bools,
            cyclomatic,
            cognitive,
            allow: Vec::new(),
            deepest_block: deepest,
            largest_block: largest,
        }
    }

    fn parse(lang: Lang, source: &str) -> Option<Tree> {
        let mut parser = Parser::new();
        parser.set_language(&lang.language()).ok()?;
        parser.parse(source, None)
    }
}

struct BodySlice<'a> {
    name: &'a str,
    kind: &'a str,
    before: &'a str,
    body: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sample {
        path: &'static str,
        src: &'static str,
    }

    fn names(sample: Sample) -> Vec<(String, String, usize, usize)> {
        let Sample { path, src } = sample;
        FnStats::extract(Path::new(path), src, Tokenizer::O200k)
            .unwrap()
            .into_iter()
            .map(|f| (f.name, f.kind, f.start_line.0, f.end_line.0))
            .collect()
    }

    #[test]
    fn rust_functions_and_methods() {
        let src = "fn a() {\n    1;\n}\n\nstruct S;\nimpl S {\n    fn b(&self) {}\n}\n";
        let got = names(Sample { path: "x.rs", src });
        assert!(got.contains(&("a".into(), "function".into(), 1, 3)));
        assert!(got.iter().any(|f| f.0 == "b" && f.2 == 7 && f.3 == 7));
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn python_functions() {
        let src = "def f():\n    return 1\n\nclass C:\n    def m(self):\n        pass\n";
        let got = names(Sample { path: "x.py", src });
        assert!(got.iter().any(|f| f.0 == "f" && f.2 == 1 && f.3 == 2));
        assert!(got.iter().any(|f| f.0 == "m" && f.2 == 5 && f.3 == 6));
    }

    #[test]
    fn typescript_loads() {
        let src = "function f(a: number): number {\n  return a;\n}\nclass C {\n  m() {}\n}\n";
        let got = names(Sample { path: "x.ts", src });
        assert!(got.iter().any(|f| f.0 == "f"));
        assert!(got.iter().any(|f| f.0 == "m"));
        assert!(!names(Sample { path: "x.tsx", src }).is_empty());
    }

    #[test]
    fn new_languages() {
        let samples = [
            ("x.java", "class C {\n  void m() {\n  }\n}\n", "m"),
            ("x.c", "int f(void) {\n  return 1;\n}\n", "f"),
            (
                "x.cpp",
                "struct S { void m() {} };\nint f() {\n  return 1;\n}\n",
                "f",
            ),
            ("x.cs", "class C {\n  C() {}\n  void M() {\n  }\n}\n", "M"),
            ("x.rb", "class C\n  def m\n  end\nend\n", "m"),
            ("x.php", "<?php\nfunction f() {\n  return 1;\n}\n", "f"),
            ("x.kt", "fun f(): Int {\n  return 1\n}\n", "f"),
            ("x.swift", "func f() -> Int {\n  return 1\n}\n", "f"),
            ("x.scala", "object O {\n  def f(): Int = 1\n}\n", "f"),
            ("x.lua", "local function f()\n  return 1\nend\n", "f"),
            ("x.ex", "defmodule M do\n  def f(a), do: a\nend\n", "f"),
            ("x.zig", "fn f() u8 {\n    return 1;\n}\n", "f"),
            ("x.sh", "f() {\n  echo hi\n}\n", "f"),
        ];
        for (path, src, name) in samples {
            let got = names(Sample { path, src });
            assert!(got.iter().any(|f| f.0 == name), "{path}: {got:?}");
        }
    }

    #[test]
    fn c_functions_span_their_body_and_skip_prototypes() {
        let src =
            "int g(int a);\nint f(int a) {\n  return a;\n}\nchar *p(void) {\n  return 0;\n}\n";
        let got = names(Sample { path: "x.c", src });
        assert_eq!(
            got,
            vec![
                ("f".into(), "function".into(), 2, 4),
                ("p".into(), "function".into(), 5, 7)
            ]
        );
    }

    #[test]
    fn unsupported_language() {
        assert!(FnStats::extract(Path::new("x.txt"), "hello", Tokenizer::O200k).is_none());
    }
}
