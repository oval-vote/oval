use crate::OvalLanguage;

/// A node in an Oval syntax tree, such as a whole document.
///
/// A node knows its parent and its position in the document, and its text is the text of every
/// token inside it.
pub type SyntaxNode = rowan::SyntaxNode<OvalLanguage>;

/// A token in an Oval syntax tree, which is a leaf of the tree.
///
/// Every token the lexer produces is in the tree, including whitespace, comments, and errors, so
/// the tokens of a tree reproduce its document exactly.
pub type SyntaxToken = rowan::SyntaxToken<OvalLanguage>;

/// Either a [`SyntaxNode`] or a [`SyntaxToken`], such as one child of a node.
pub type SyntaxElement = rowan::SyntaxElement<OvalLanguage>;
