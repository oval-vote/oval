use rowan::GreenNode;

use crate::{LexicalError, SyntaxError, SyntaxNode};

/// The syntax tree and errors of a document, as returned by [`parse`](crate::parse).
///
/// A document with errors still parses completely, so the tree always covers the whole document and
/// the errors are reported alongside it rather than in place of it. A document with no errors of
/// either kind is well-formed in the sense of the specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub(super) green: GreenNode,
    /// The lexical errors, in document order, as [`lex`](crate::lex) reports them.
    pub lexical_errors: Vec<LexicalError>,
    /// The syntax errors, in document order. Empty when the tokens match the grammar.
    pub syntax_errors: Vec<SyntaxError>,
}

impl ParseResult {
    /// Returns the root of the syntax tree, a [`Document`](crate::SyntaxKind::Document) node whose
    /// text is the whole document.
    ///
    /// Each call returns a new handle to the same tree, which is cheap, so call it whenever you need
    /// the tree rather than keeping the node around.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
}
