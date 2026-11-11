mod token_kind;

pub use token_kind::TokenKind;

/// A token in an Oval document
///
/// A token holds no text of its own. Tokens are contiguous, so a token's text is the slice of the
/// document that begins where previous token ends. [`TokenKind::Indent`] and
/// [`TokenKind::Dedent`] tokens, and a [`TokenKind::Newline`] at the end of input, have no text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// The kind of the token.
    pub kind: TokenKind,
    /// The length of the token's text in bytes.
    pub len: u32,
}

impl Token {
    /// Returns `true` if the token is ignored by the parser. See [`TokenKind::is_ignored`].
    pub fn is_ignored(self) -> bool {
        self.kind.is_ignored()
    }

    /// Returns `true` if the token is a layout token. See [`TokenKind::is_layout`].
    pub fn is_layout(self) -> bool {
        self.kind.is_layout()
    }

    /// Returns `true` if the token is a lexical token. See [`TokenKind::is_lexical`].
    pub fn is_lexical(self) -> bool {
        self.kind.is_lexical()
    }

    /// Returns `true` if the token is an error token. See [`TokenKind::is_error`].
    pub fn is_error(self) -> bool {
        self.kind.is_error()
    }
}
