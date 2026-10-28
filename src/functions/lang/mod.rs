//! Supported languages: file extensions, grammars, tags queries and node-kind tables.

mod grammar;
mod tags;

use std::path::Path;

use tree_sitter::{Language, Node};

pub(super) use grammar::{Grammar, ParamSpec};

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
    Kotlin,
    Swift,
    Scala,
    Lua,
    Elixir,
    Zig,
    Bash,
}

impl Lang {
    pub(super) const ALL: [Self; 19] = [
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
        Self::Kotlin,
        Self::Swift,
        Self::Scala,
        Self::Lua,
        Self::Elixir,
        Self::Zig,
        Self::Bash,
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
            "kt" | "kts" => Some(Self::Kotlin),
            "swift" => Some(Self::Swift),
            "scala" | "sc" => Some(Self::Scala),
            "lua" => Some(Self::Lua),
            "ex" | "exs" => Some(Self::Elixir),
            "zig" => Some(Self::Zig),
            "sh" | "bash" => Some(Self::Bash),
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
            Self::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
            Self::Swift => tree_sitter_swift::LANGUAGE.into(),
            Self::Scala => tree_sitter_scala::LANGUAGE.into(),
            Self::Lua => tree_sitter_lua::LANGUAGE.into(),
            Self::Elixir => tree_sitter_elixir::LANGUAGE.into(),
            Self::Zig => tree_sitter_zig::LANGUAGE.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
        }
    }

    /// The node's kind, except that a call to a control-flow keyword (Elixir's `if`, `case`,
    /// ...) reads as the keyword, so the kind tables can name it.
    pub(super) fn kind_of<'a>(self, node: Node<'a>, source: &'a str) -> &'a str {
        let calls = self.grammar().keyword_calls;
        if calls.is_empty() || node.kind() != "call" {
            return node.kind();
        }
        node.child_by_field_name("target")
            .filter(|target| target.kind() == "identifier")
            .and_then(|target| source.get(target.byte_range()))
            .and_then(|name| calls.iter().find(|call| **call == name).copied())
            .unwrap_or(node.kind())
    }
}
