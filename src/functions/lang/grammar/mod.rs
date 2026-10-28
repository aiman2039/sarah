//! Node-kind tables per language. Every kind is checked against its grammar by
//! `kinds_exist`, so a typo fails a test instead of silently never matching.

mod managed;
mod scripting;
mod systems;

use managed::{CSHARP, JAVA, KOTLIN, SCALA, SWIFT};
use scripting::{BASH, ELIXIR, JAVASCRIPT, LUA, PHP, PYTHON, RUBY, TYPESCRIPT};
use systems::{C, CPP, GO, RUST, ZIG};

use super::Lang;

/// Where a function's parameters are, and which children of that list count.
pub(in crate::functions) struct ParamSpec {
    /// Parameter-list kinds; empty when parameters are direct children of the definition.
    pub lists: &'static [&'static str],
    /// Kinds of one parameter.
    pub items: &'static [&'static str],
    /// One item may declare several names (Go's `a, b int`); count its `name` fields.
    pub names_per_item: bool,
}

pub(in crate::functions) struct Grammar {
    /// Opens one level of control-flow nesting (`depth`).
    pub nesting: &'static [&'static str],
    /// `if`, loops, `catch`, ternaries: +1 to both complexities, nesting-weighted in cognitive.
    pub branches: &'static [&'static str],
    /// `elif`-style clauses: +1 to both complexities, never nesting-weighted.
    pub elifs: &'static [&'static str],
    /// `switch`/`match`: +1 to cognitive (nesting-weighted); cyclomatic counts the arms instead.
    pub switches: &'static [&'static str],
    /// Arms of a `switch`/`match`: +1 to cyclomatic each.
    pub cases: &'static [&'static str],
    /// Binary nodes; logical when the operator is `&&`, `||`, `and` or `or`.
    pub binaries: &'static [&'static str],
    /// `None` when sarah cannot tell parameters apart (Elixir, Bash).
    pub params: Option<ParamSpec>,
    /// Calls that are control flow (Elixir); tables name them by the called keyword.
    pub keyword_calls: &'static [&'static str],
}

impl Lang {
    pub(in crate::functions) fn grammar(self) -> &'static Grammar {
        match self {
            Self::Rust => &RUST,
            Self::Python => &PYTHON,
            Self::JavaScript => &JAVASCRIPT,
            Self::TypeScript | Self::Tsx => &TYPESCRIPT,
            Self::Go => &GO,
            Self::Java => &JAVA,
            Self::C => &C,
            Self::Cpp => &CPP,
            Self::CSharp => &CSHARP,
            Self::Ruby => &RUBY,
            Self::Php => &PHP,
            Self::Kotlin => &KOTLIN,
            Self::Swift => &SWIFT,
            Self::Scala => &SCALA,
            Self::Lua => &LUA,
            Self::Elixir => &ELIXIR,
            Self::Zig => &ZIG,
            Self::Bash => &BASH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_exist() {
        for lang in Lang::ALL {
            let language = lang.language();
            let g = lang.grammar();
            let params = g.params.iter().flat_map(|p| p.lists.iter().chain(p.items));
            let kinds = [
                g.nesting, g.branches, g.elifs, g.switches, g.cases, g.binaries,
            ]
            .into_iter()
            .flatten()
            .chain(params)
            .filter(|kind| !g.keyword_calls.contains(kind));
            for kind in kinds {
                assert_ne!(language.id_for_node_kind(kind, true), 0, "{lang:?} {kind}");
            }
        }
    }
}
