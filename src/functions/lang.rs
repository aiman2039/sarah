use std::path::Path;
use std::sync::OnceLock;

use tree_sitter_tags::TagsConfiguration;

#[derive(Clone, Copy)]
pub(super) enum Lang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
}

impl Lang {
    pub(super) fn from_path(path: &Path) -> Option<Self> {
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

    pub(super) fn config(self) -> Option<&'static TagsConfiguration> {
        type Cell = OnceLock<Option<TagsConfiguration>>;
        static RUST: Cell = OnceLock::new();
        static PYTHON: Cell = OnceLock::new();
        static JAVASCRIPT: Cell = OnceLock::new();
        static TYPESCRIPT: Cell = OnceLock::new();
        static TSX: Cell = OnceLock::new();
        static GO: Cell = OnceLock::new();

        // TypeScript's tags query only adds TS-specific patterns on top of JavaScript's.
        let ts_query = || {
            format!(
                "{}\n{}",
                tree_sitter_javascript::TAGS_QUERY,
                tree_sitter_typescript::TAGS_QUERY
            )
        };
        let cell = match self {
            Self::Rust => &RUST,
            Self::Python => &PYTHON,
            Self::JavaScript => &JAVASCRIPT,
            Self::TypeScript => &TYPESCRIPT,
            Self::Tsx => &TSX,
            Self::Go => &GO,
        };
        cell.get_or_init(|| {
            let (language, query): (tree_sitter::Language, String) = match self {
                Self::Rust => (
                    tree_sitter_rust::LANGUAGE.into(),
                    tree_sitter_rust::TAGS_QUERY.into(),
                ),
                Self::Python => (
                    tree_sitter_python::LANGUAGE.into(),
                    tree_sitter_python::TAGS_QUERY.into(),
                ),
                Self::JavaScript => (
                    tree_sitter_javascript::LANGUAGE.into(),
                    tree_sitter_javascript::TAGS_QUERY.into(),
                ),
                Self::TypeScript => (
                    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                    ts_query(),
                ),
                Self::Tsx => (tree_sitter_typescript::LANGUAGE_TSX.into(), ts_query()),
                Self::Go => (
                    tree_sitter_go::LANGUAGE.into(),
                    tree_sitter_go::TAGS_QUERY.into(),
                ),
            };
            // The queries ship with the grammar crates; `all_tags_queries_load` checks them.
            TagsConfiguration::new(language, &query, "").ok()
        })
        .as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tags_queries_load() {
        for lang in [
            Lang::Rust,
            Lang::Python,
            Lang::JavaScript,
            Lang::TypeScript,
            Lang::Tsx,
            Lang::Go,
        ] {
            assert!(lang.config().is_some());
        }
    }
}
