use crate::OvalLanguage;

/// A token in an Oval syntax tree, which is a leaf of the tree.
///
/// Every token the lexer produces is in the tree, including whitespace, comments, and errors, so
/// the tokens of a tree reproduce its document exactly.
pub type SyntaxToken = rowan::SyntaxToken<OvalLanguage>;
