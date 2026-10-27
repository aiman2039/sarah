use std::path::Path;
use std::sync::OnceLock;

use tree_sitter::Language;
use tree_sitter_tags::TagsConfiguration;

/// Definitions only: sarah reads nothing else from tags.
const CSHARP_TAGS_QUERY: &str = "
(method_declaration name: (identifier) @name) @definition.method
(constructor_declaration name: (identifier) @name) @definition.method
(local_function_statement name: (identifier) @name) @definition.function
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Ruby,
    Php,
}

impl Lang {
    pub(super) const ALL: [Self; 12] = [
        Self::Rust,
        Self::Python,
        Self::JavaScript,
        Self::TypeScript,
        Self::Tsx,
        Self::Go,
        Self::Java,
        Self::C,
        Self::Cpp,
        Self::CSharp,
        Self::Ruby,
        Self::Php,
    ];

    pub(super) fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "c" | "h" => Some(Self::C),
            "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "h++" => Some(Self::Cpp),
            "cs" => Some(Self::CSharp),
            "rb" => Some(Self::Ruby),
            "php" => Some(Self::Php),
            _ => None,
        }
    }

    pub(super) fn language(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        }
    }

    /// Node kinds that open one level of control-flow nesting.
    pub(super) fn nesting_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &[
                "if_expression",
                "match_expression",
                "for_expression",
                "while_expression",
                "loop_expression",
            ],
            Self::Python => &[
                "if_statement",
                "for_statement",
                "while_statement",
                "try_statement",
                "with_statement",
                "match_statement",
            ],
            Self::JavaScript | Self::TypeScript | Self::Tsx => &[
                "if_statement",
                "for_statement",
                "for_in_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
                "try_statement",
            ],
            Self::Go => &[
                "if_statement",
                "for_statement",
                "expression_switch_statement",
                "type_switch_statement",
                "select_statement",
            ],
            Self::Java => &[
                "if_statement",
                "for_statement",
                "enhanced_for_statement",
                "while_statement",
                "do_statement",
                "switch_expression",
                "try_statement",
            ],
            Self::C => &[
                "if_statement",
                "for_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
            ],
            Self::Cpp => &[
                "if_statement",
                "for_statement",
                "for_range_loop",
                "while_statement",
                "do_statement",
                "switch_statement",
                "try_statement",
            ],
            Self::CSharp => &[
                "if_statement",
                "for_statement",
                "foreach_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
                "try_statement",
            ],
            Self::Ruby => &["if", "unless", "while", "until", "for", "case", "begin"],
            Self::Php => &[
                "if_statement",
                "for_statement",
                "foreach_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
                "try_statement",
            ],
        }
    }

    pub(super) fn config(self) -> Option<&'static TagsConfiguration> {
        type Cell = OnceLock<Option<TagsConfiguration>>;
        static CELLS: [Cell; 12] = [const { OnceLock::new() }; 12];

        let index = Self::ALL.iter().position(|l| *l == self)?;
        CELLS
            .get(index)?
            .get_or_init(|| {
                // The queries ship with the grammar crates; `all_tags_queries_load` checks them.
                TagsConfiguration::new(self.language(), &self.tags_query(), "").ok()
            })
            .as_ref()
    }

    fn tags_query(self) -> String {
        match self {
            Self::Rust => tree_sitter_rust::TAGS_QUERY.into(),
            Self::Python => tree_sitter_python::TAGS_QUERY.into(),
            Self::JavaScript => tree_sitter_javascript::TAGS_QUERY.into(),
            // TypeScript's tags query only adds TS-specific patterns on top of JavaScript's.
            Self::TypeScript | Self::Tsx => format!(
                "{}\n{}",
                tree_sitter_javascript::TAGS_QUERY,
                tree_sitter_typescript::TAGS_QUERY
            ),
            Self::Go => tree_sitter_go::TAGS_QUERY.into(),
            Self::Java => tree_sitter_java::TAGS_QUERY.into(),
            Self::C => tree_sitter_c::TAGS_QUERY.into(),
            Self::Cpp => tree_sitter_cpp::TAGS_QUERY.into(),
            // The crate's own tags query does not compile against its grammar (0.23.5).
            Self::CSharp => CSHARP_TAGS_QUERY.into(),
            Self::Ruby => tree_sitter_ruby::TAGS_QUERY.into(),
            Self::Php => tree_sitter_php::TAGS_QUERY.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tags_queries_load() {
        for lang in Lang::ALL {
            assert!(lang.config().is_some(), "{lang:?}");
        }
    }

    /// A typo in a nesting kind would silently never match.
    #[test]
    fn nesting_kinds_exist() {
        for lang in Lang::ALL {
            let language = lang.language();
            for kind in lang.nesting_kinds() {
                assert_ne!(language.id_for_node_kind(kind, true), 0, "{lang:?} {kind}");
            }
        }
    }
}
