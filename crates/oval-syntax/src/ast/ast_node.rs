use crate::{SyntaxKind, SyntaxNode};

/// A typed view of a [`SyntaxNode`] of one kind, such as a [`Document`](super::Document) for a
/// [`Document`](SyntaxKind::Document) node.
pub trait AstNode: Sized {
    /// Returns `true` if a node of `kind` can be viewed as this type.
    fn can_cast(kind: SyntaxKind) -> bool;

    /// Returns a view of `node`, or `None` if a node of its kind cannot be viewed as this type.
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// Returns the node this view wraps.
    fn syntax(&self) -> &SyntaxNode;
}

macro_rules! ast_node {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name($crate::SyntaxNode);

        impl $crate::ast::AstNode for $name {
            fn can_cast(kind: $crate::SyntaxKind) -> bool {
                kind == $crate::SyntaxKind::$name
            }

            fn cast(node: $crate::SyntaxNode) -> Option<Self> {
                Self::can_cast(node.kind()).then(|| Self(node))
            }

            fn syntax(&self) -> &$crate::SyntaxNode {
                &self.0
            }
        }
    };
}

pub(crate) use ast_node;
