use crate::{LexicalError, Token};

/// The tokens and lexical errors of a document, as returned by [`lex`](crate::lex) and
/// [`lex_bytes`](crate::lex_bytes).
///
/// As per the spev, an Oval document with lexical errors still lexes completely, so the tokens always cover the whole
/// document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexResult {
    /// The tokens, in document order. Their lengths add up to the length of the document.
    pub tokens: Vec<Token>,
    /// The lexical errors, in document order. Empty when the document has none.
    pub errors: Vec<LexicalError>,
}
