//! Public API items per file: functions, methods and types other files can use.
//! Languages whose visibility sarah cannot read from the syntax tree get `None`.

mod exports;
mod keywords;
#[cfg(test)]
mod tests;

use tree_sitter::Node;

use crate::lang::Lang;
use crate::lang::Scope;
use crate::units::Items;

/// Which nodes are API items and how many public items one of them adds.
pub(super) struct VisibilitySpec {
    /// Kinds that may be API items; `count` decides how many of them are public.
    pub items: &'static [&'static str],
    /// Kinds whose subtrees are never API (function bodies, initializers).
    pub opaque: &'static [&'static str],
    /// Public items the node adds (0 when it is not public).
    pub count: fn(&Scope, Node) -> usize,
}

/// Modifier node kinds whose text is a visibility word (`pub`, `public`, `private[x]`).
const MODIFIER_KINDS: &[&str] = &["modifier", "visibility_modifier", "access_modifier", "pub"];

/// Stateless helpers that count a file's public API items.
pub(crate) struct VisibilityHelpers;

impl VisibilityHelpers {
    /// `None` for languages without a visibility rule (C, C++, Ruby, Lua, Elixir, Bash).
    pub(crate) fn count(root: &Scope) -> Option<Items> {
        let spec = Self::spec(root.lang)?;
        let mut total = 0usize;
        let mut stack = vec![root.node];
        while let Some(parent) = stack.pop() {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                if spec.items.contains(&child.kind()) {
                    total = total.saturating_add((spec.count)(root, child));
                }
                if !spec.opaque.contains(&child.kind()) {
                    stack.push(child);
                }
            }
        }
        Some(Items(total))
    }

    /// Visibility words on `node`: its own modifier children and the children of its
    /// `modifiers` child.
    pub(super) fn modifiers<'a>(scope: &Scope<'a>, node: Node<'a>) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let mut inner = child.walk();
                out.extend(child.children(&mut inner).map(|m| scope.text(m)));
            } else if MODIFIER_KINDS.contains(&child.kind()) {
                out.push(scope.text(child));
            }
        }
        out
    }

    /// Whether any visibility word on `node` starts with one of `words`.
    pub(super) fn has_any(scope: &Scope, node: Node, words: &[&str]) -> bool {
        Self::modifiers(scope, node)
            .iter()
            .any(|m| words.iter().any(|w| m.starts_with(w)))
    }

    fn spec(lang: Lang) -> Option<&'static VisibilitySpec> {
        Some(match lang {
            Lang::Rust => &keywords::RUST,
            Lang::Java => &keywords::JAVA,
            Lang::CSharp => &keywords::CSHARP,
            Lang::Kotlin => &keywords::KOTLIN,
            Lang::Swift => &keywords::SWIFT,
            Lang::Scala => &keywords::SCALA,
            Lang::Php => &keywords::PHP,
            Lang::Zig => &keywords::ZIG,
            Lang::Go => &exports::GO,
            Lang::Python => &exports::PYTHON,
            Lang::JavaScript | Lang::TypeScript | Lang::Tsx => &exports::JS,
            Lang::C | Lang::Cpp | Lang::Ruby | Lang::Lua | Lang::Elixir | Lang::Bash => {
                return None;
            }
        })
    }
}
