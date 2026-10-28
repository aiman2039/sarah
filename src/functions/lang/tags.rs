use std::sync::OnceLock;

use tree_sitter_tags::TagsConfiguration;

use super::Lang;

// Definitions only: sarah reads nothing else from tags.

/// The C# crate's own tags query does not compile against its grammar (0.23.5).
const CSHARP_TAGS_QUERY: &str = "
(method_declaration name: (identifier) @name) @definition.method
(constructor_declaration name: (identifier) @name) @definition.method
(local_function_statement name: (identifier) @name) @definition.function
";

/// C's and C++'s own queries tag the declarator only (no body, prototypes included).
const C_TAGS_QUERY: &str = "
(function_definition declarator: (function_declarator declarator: (identifier) @name)) @definition.function
(function_definition declarator: (pointer_declarator declarator: (function_declarator declarator: (identifier) @name))) @definition.function
(function_definition declarator: (pointer_declarator declarator: (pointer_declarator declarator: (function_declarator declarator: (identifier) @name)))) @definition.function
";
const CPP_TAGS_QUERY: &str = "
(function_definition declarator: (function_declarator declarator: [(identifier) (field_identifier) (destructor_name) (operator_name)] @name)) @definition.function
(function_definition declarator: (function_declarator declarator: (qualified_identifier name: (_) @name))) @definition.method
(function_definition declarator: (pointer_declarator declarator: (function_declarator declarator: [(identifier) (field_identifier) (qualified_identifier)] @name))) @definition.function
(function_definition declarator: (reference_declarator (function_declarator declarator: [(identifier) (field_identifier) (qualified_identifier)] @name))) @definition.function
";

/// These grammar crates ship no tags query.
const KOTLIN_TAGS_QUERY: &str = "
(function_declaration name: (identifier) @name) @definition.function
";
const SCALA_TAGS_QUERY: &str = "
(function_definition name: (_) @name) @definition.function
";
const ZIG_TAGS_QUERY: &str = "
(function_declaration name: (identifier) @name) @definition.function
";
const BASH_TAGS_QUERY: &str = "
(function_definition name: (word) @name) @definition.function
";

/// Swift's own query tags whole class bodies as methods; keep the declarations only.
const SWIFT_TAGS_QUERY: &str = "
(function_declaration name: (simple_identifier) @name) @definition.function
";

impl Lang {
    pub(in crate::functions) fn config(self) -> Option<&'static TagsConfiguration> {
        type Cell = OnceLock<Option<TagsConfiguration>>;
        static CELLS: [Cell; Lang::ALL.len()] = [const { OnceLock::new() }; Lang::ALL.len()];

        let index = Self::ALL.iter().position(|l| *l == self)?;
        CELLS
            .get(index)?
            .get_or_init(|| {
                // `all_tags_queries_load` checks that every query compiles.
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
            Self::C => C_TAGS_QUERY.into(),
            Self::Cpp => CPP_TAGS_QUERY.into(),
            Self::CSharp => CSHARP_TAGS_QUERY.into(),
            Self::Ruby => tree_sitter_ruby::TAGS_QUERY.into(),
            Self::Php => tree_sitter_php::TAGS_QUERY.into(),
            Self::Kotlin => KOTLIN_TAGS_QUERY.into(),
            Self::Swift => SWIFT_TAGS_QUERY.into(),
            Self::Scala => SCALA_TAGS_QUERY.into(),
            Self::Lua => tree_sitter_lua::TAGS_QUERY.into(),
            Self::Elixir => tree_sitter_elixir::TAGS_QUERY.into(),
            Self::Zig => ZIG_TAGS_QUERY.into(),
            Self::Bash => BASH_TAGS_QUERY.into(),
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
}
