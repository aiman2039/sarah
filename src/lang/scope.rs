use tree_sitter::Node;

use super::{Grammar, Lang};

/// One function definition in its parsed file: what every per-function metric walks.
#[derive(Clone, Copy)]
pub(crate) struct Scope<'a> {
    pub lang: Lang,
    pub node: Node<'a>,
    pub source: &'a str,
}

impl<'a> Scope<'a> {
    pub(crate) fn grammar(&self) -> &'static Grammar {
        self.lang.grammar()
    }

    pub(crate) fn kind_of(&self, node: Node<'a>) -> &'a str {
        self.lang.kind_of(node, self.source)
    }

    pub(crate) fn text(&self, node: Node) -> &'a str {
        self.source.get(node.byte_range()).unwrap_or_default()
    }

    /// `else if` continues a chain; it is not one level deeper.
    pub(crate) fn is_else_if(child: Node, parent: Node) -> bool {
        child.kind().contains("if")
            && (parent.kind() == child.kind() || matches!(parent.kind(), "else_clause" | "else"))
    }
}
