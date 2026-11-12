use crate::OvalLanguage;

/// Either a [`SyntaxNode`](crate::SyntaxNode) or a [`SyntaxToken`](crate::SyntaxToken), such as
/// one child of a node.
pub type SyntaxElement = rowan::SyntaxElement<OvalLanguage>;
