use crate::{LexicalError, Token};

/// The result of a lexing operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexResult {
    /// The tokens
    pub tokens: Vec<Token>,
    /// The lexical errors, if any
    pub errors: Vec<LexicalError>,
}
