use crate::OvalLanguage;

/// A node in an Oval syntax tree, such as a whole document.
///
/// A node knows its parent and its position in the document, and its text is the text of every
/// token inside it.
pub type SyntaxNode = rowan::SyntaxNode<OvalLanguage>;
