use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use serde::Serialize;
use tree_sitter_tags::{TagsConfiguration, TagsContext};

use crate::count::Tokenizer;

#[derive(Debug, Clone, Serialize)]
pub struct FnStats {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub lines: usize,
    pub tokens: usize,
}

#[derive(Clone, Copy)]
enum Lang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
}

impl Lang {
    fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    fn config(self) -> Option<&'static TagsConfiguration> {
        static CONFIGS: [OnceLock<Option<TagsConfiguration>>; 6] =
            [const { OnceLock::new() }; 6];
        CONFIGS[self as usize]
            .get_or_init(|| {
                // TypeScript's tags query only adds TS-specific patterns on top of JavaScript's.
                let ts_query = format!(
                    "{}\n{}",
                    tree_sitter_javascript::TAGS_QUERY,
                    tree_sitter_typescript::TAGS_QUERY
                );
                let (language, query): (tree_sitter::Language, &str) = match self {
                    Self::Rust => (tree_sitter_rust::LANGUAGE.into(), tree_sitter_rust::TAGS_QUERY),
                    Self::Python => (
                        tree_sitter_python::LANGUAGE.into(),
                        tree_sitter_python::TAGS_QUERY,
                    ),
                    Self::JavaScript => (
                        tree_sitter_javascript::LANGUAGE.into(),
                        tree_sitter_javascript::TAGS_QUERY,
                    ),
                    Self::TypeScript => {
                        (tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), &ts_query)
                    }
                    Self::Tsx => (tree_sitter_typescript::LANGUAGE_TSX.into(), &ts_query),
                    Self::Go => (tree_sitter_go::LANGUAGE.into(), tree_sitter_go::TAGS_QUERY),
                };
                TagsConfiguration::new(language, query, "")
                    .inspect_err(|e| eprintln!("warning: tags query failed to load: {e}"))
                    .ok()
            })
            .as_ref()
    }
}

/// Extract function/method definitions with their sizes. Returns `None` for unsupported languages.
pub fn extract(path: &Path, source: &str, tokenizer: Tokenizer) -> Option<Vec<FnStats>> {
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
        let body = &source[tag.range.clone()];
        let start_line = line_of(source, tag.range.start);
        let end_line = start_line + body.matches('\n').count();
        out.push(FnStats {
            name: source[tag.name_range].to_string(),
            kind: kind.to_string(),
            start_line,
            end_line,
            lines: end_line - start_line + 1,
            tokens: tokenizer.count(body),
        });
    }
    Some(out)
}

fn line_of(source: &str, byte: usize) -> usize {
    source[..byte].matches('\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(path: &str, src: &str) -> Vec<(String, String, usize, usize)> {
        extract(Path::new(path), src, Tokenizer::O200k)
            .unwrap()
            .into_iter()
            .map(|f| (f.name, f.kind, f.start_line, f.end_line))
            .collect()
    }

    #[test]
    fn rust_functions_and_methods() {
        let src = "fn a() {\n    1;\n}\n\nstruct S;\nimpl S {\n    fn b(&self) {}\n}\n";
        let got = names("x.rs", src);
        assert!(got.contains(&("a".into(), "function".into(), 1, 3)));
        assert!(got.iter().any(|f| f.0 == "b" && f.2 == 7 && f.3 == 7));
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn python_functions() {
        let src = "def f():\n    return 1\n\nclass C:\n    def m(self):\n        pass\n";
        let got = names("x.py", src);
        assert!(got.iter().any(|f| f.0 == "f" && f.2 == 1 && f.3 == 2));
        assert!(got.iter().any(|f| f.0 == "m" && f.2 == 5 && f.3 == 6));
    }

    #[test]
    fn typescript_loads() {
        let src = "function f(a: number): number {\n  return a;\n}\nclass C {\n  m() {}\n}\n";
        let got = names("x.ts", src);
        assert!(got.iter().any(|f| f.0 == "f"));
        assert!(got.iter().any(|f| f.0 == "m"));
        assert!(!names("x.tsx", src).is_empty());
    }

    #[test]
    fn unsupported_language() {
        assert!(extract(Path::new("x.txt"), "hello", Tokenizer::O200k).is_none());
    }
}
