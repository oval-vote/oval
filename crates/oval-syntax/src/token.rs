use crate::TokenKind;

/// A token in an Oval document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The length of the token in bytes
    pub len: u32,
}
