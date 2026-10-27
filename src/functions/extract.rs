use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;
use tree_sitter_tags::TagsContext;

use super::lang::Lang;
use crate::count::Tokenizer;
use crate::units::{LineNo, Lines, Tokens};

#[derive(Debug, Clone, Serialize)]
pub struct FnStats {
    pub name: String,
    pub kind: String,
    pub start_line: LineNo,
    pub end_line: LineNo,
    pub lines: Lines,
    pub tokens: Tokens,
}

impl FnStats {
    /// Extract function/method definitions with their sizes. Returns `None` for unsupported languages.
    pub fn extract(path: &Path, source: &str, tokenizer: Tokenizer) -> Option<Vec<Self>> {
        let config = Lang::from_path(path)?.config()?;
        let mut ctx = TagsContext::new();
        let (tags, _) = ctx.generate_tags(config, source.as_bytes(), None).ok()?;

        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for tag in tags.filter_map(Result::ok) {
            let kind = config.syntax_type_name(tag.syntax_type_id);
            if !tag.is_definition || !matches!(kind, "function" | "method") {
                continue;
            }
            // Rust methods match both the method and function patterns; keep the first.
            if !seen.insert(tag.range.clone()) {
                continue;
            }
            // Tag ranges come from the parser and land on UTF-8 boundaries; skip any that don't.
            let (Some(body), Some(name), Some(before)) = (
                source.get(tag.range.clone()),
                source.get(tag.name_range),
                source.get(..tag.range.start),
            ) else {
                continue;
            };
            let start_line = LineNo(1).below(before.matches('\n').count());
            let end_line = start_line.below(body.matches('\n').count());
            out.push(Self {
                name: name.to_string(),
                kind: kind.to_string(),
                start_line,
                end_line,
                lines: start_line.span_to(end_line),
                tokens: tokenizer.count(body),
            });
        }
        Some(out)
    }
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
    fn unsupported_language() {
        assert!(FnStats::extract(Path::new("x.txt"), "hello", Tokenizer::O200k).is_none());
    }
}
