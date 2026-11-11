mod token_kind;

pub use token_kind::TokenKind;

/// A token in an Oval document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The length of the token in bytes
    pub len: u32,
}

impl Token {
    /// Whether the token is ignored.
    pub fn is_ignored(self) -> bool {
        self.kind.is_ignored()
    }

    /// Whether the token is a layout token.
    pub fn is_layout(self) -> bool {
        self.kind.is_layout()
    }

    /// Whether the token is a lexical token.
    pub fn is_lexical(self) -> bool {
        self.kind.is_lexical()
    }

    /// Whether the token is an error token.
    pub fn is_error(self) -> bool {
        self.kind.is_error()
    }
}

#[cfg(test)]
mod tests {
    use super::token_kind::ALL_KINDS;
    use super::*;

    #[test]
    fn a_token_is_in_the_category_of_its_kind() {
        for kind in ALL_KINDS {
            let token = Token { kind, len: 0 };
            assert_eq!(token.is_lexical(), kind.is_lexical(), "{kind:?}");
            assert_eq!(token.is_ignored(), kind.is_ignored(), "{kind:?}");
            assert_eq!(token.is_layout(), kind.is_layout(), "{kind:?}");
            assert_eq!(token.is_error(), kind.is_error(), "{kind:?}");
        }
    }
}
