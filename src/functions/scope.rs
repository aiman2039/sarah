use tree_sitter::Node;

use super::lang::{Grammar, Lang};

/// One function definition in its parsed file: what every per-function metric walks.
#[derive(Clone, Copy)]
pub(super) struct Scope<'a> {
    pub lang: Lang,
    pub node: Node<'a>,
    pub source: &'a str,
}

impl<'a> Scope<'a> {
    pub(super) fn grammar(&self) -> &'static Grammar {
        self.lang.grammar()
    }

    pub(super) fn kind_of(&self, node: Node<'a>) -> &'a str {
        self.lang.kind_of(node, self.source)
    }

    pub(super) fn text(&self, node: Node) -> &'a str {
        self.source.get(node.byte_range()).unwrap_or_default()
    }

    /// `else if` continues a chain; it is not one level deeper.
    pub(super) fn is_else_if(child: Node, parent: Node) -> bool {
        child.kind().contains("if")
            && (parent.kind() == child.kind() || matches!(parent.kind(), "else_clause" | "else"))
    }
}
