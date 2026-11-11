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

/// The kinds of tokens in an Oval document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A word token
    Word,
    /// A string token
    String,
    /// An integer token
    Integer,
    /// A date token
    Date,
    /// An at sign
    AtSign,
    /// A URL
    Url,
    /// A colon
    Colon,
    /// A comma
    Comma,
    /// A markdown block
    MarkdownBlock,
    /// A white space
    Whitespace,
    /// A comment
    Comment,
    /// A byte order mark (U+FEFF) at the very start of a document, which is ignored.
    ByteOrderMark,
    /// A blank line
    BlankLine,
    /// A new line
    Newline,
    /// An indent
    Indent,
    /// A dedent
    Dedent,
    /// An error
    Error,
}

impl TokenKind {
    /// Whether the token is ignored.
    pub fn is_ignored(self) -> bool {
        match self {
            Self::Whitespace | Self::Comment | Self::BlankLine | Self::ByteOrderMark => true,
            Self::Word
            | Self::String
            | Self::Integer
            | Self::Date
            | Self::AtSign
            | Self::Url
            | Self::Colon
            | Self::Comma
            | Self::MarkdownBlock
            | Self::Newline
            | Self::Indent
            | Self::Dedent
            | Self::Error => false,
        }
    }

    /// Whether the token is a layout token.
    pub fn is_layout(self) -> bool {
        match self {
            Self::Newline | Self::Indent | Self::Dedent => true,
            Self::Whitespace
            | Self::Comment
            | Self::BlankLine
            | Self::ByteOrderMark
            | Self::Word
            | Self::String
            | Self::Integer
            | Self::Date
            | Self::AtSign
            | Self::Url
            | Self::Colon
            | Self::Comma
            | Self::MarkdownBlock
            | Self::Error => false,
        }
    }

    /// Whether the token is a lexical token.
    pub fn is_lexical(self) -> bool {
        match self {
            Self::Word
            | Self::String
            | Self::Integer
            | Self::Date
            | Self::AtSign
            | Self::Url
            | Self::Colon
            | Self::Comma
            | Self::MarkdownBlock => true,
            Self::Whitespace
            | Self::Comment
            | Self::BlankLine
            | Self::ByteOrderMark
            | Self::Newline
            | Self::Indent
            | Self::Dedent
            | Self::Error => false,
        }
    }

    /// Whether the token is an error token.
    pub fn is_error(self) -> bool {
        self == Self::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_KINDS: [TokenKind; 17] = [
        TokenKind::Word,
        TokenKind::String,
        TokenKind::Integer,
        TokenKind::Date,
        TokenKind::AtSign,
        TokenKind::Url,
        TokenKind::Colon,
        TokenKind::Comma,
        TokenKind::MarkdownBlock,
        TokenKind::Whitespace,
        TokenKind::Comment,
        TokenKind::ByteOrderMark,
        TokenKind::BlankLine,
        TokenKind::Newline,
        TokenKind::Indent,
        TokenKind::Dedent,
        TokenKind::Error,
    ];

    // Fails to compile when a kind is added, as a reminder to add it to `ALL_KINDS` too.
    #[allow(dead_code)]
    fn every_kind_is_in_all_kinds(kind: TokenKind) {
        match kind {
            TokenKind::Word
            | TokenKind::String
            | TokenKind::Integer
            | TokenKind::Date
            | TokenKind::AtSign
            | TokenKind::Url
            | TokenKind::Colon
            | TokenKind::Comma
            | TokenKind::MarkdownBlock
            | TokenKind::Whitespace
            | TokenKind::Comment
            | TokenKind::ByteOrderMark
            | TokenKind::BlankLine
            | TokenKind::Newline
            | TokenKind::Indent
            | TokenKind::Dedent
            | TokenKind::Error => {}
        }
    }

    // Asserts that `predicate` holds for exactly the kinds in `expected`.
    fn check_category(name: &str, predicate: fn(TokenKind) -> bool, expected: &[TokenKind]) {
        for kind in ALL_KINDS {
            assert_eq!(
                predicate(kind),
                expected.contains(&kind),
                "{name} of {kind:?}"
            );
        }
    }

    #[test]
    fn lexical_tokens_are_the_specification_tokens() {
        check_category(
            "is_lexical",
            TokenKind::is_lexical,
            &[
                TokenKind::Word,
                TokenKind::String,
                TokenKind::Integer,
                TokenKind::Date,
                TokenKind::AtSign,
                TokenKind::Url,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::MarkdownBlock,
            ],
        );
    }

    #[test]
    fn ignored_tokens_are_whitespace_comments_blank_lines_and_byte_order_marks() {
        check_category(
            "is_ignored",
            TokenKind::is_ignored,
            &[
                TokenKind::Whitespace,
                TokenKind::Comment,
                TokenKind::BlankLine,
                TokenKind::ByteOrderMark,
            ],
        );
    }

    #[test]
    fn layout_tokens_are_newline_indent_and_dedent() {
        check_category(
            "is_layout",
            TokenKind::is_layout,
            &[TokenKind::Newline, TokenKind::Indent, TokenKind::Dedent],
        );
    }

    #[test]
    fn only_error_is_an_error() {
        check_category("is_error", TokenKind::is_error, &[TokenKind::Error]);
    }

    #[test]
    fn every_kind_is_in_exactly_one_category() {
        for kind in ALL_KINDS {
            let categories = [
                kind.is_lexical(),
                kind.is_ignored(),
                kind.is_layout(),
                kind.is_error(),
            ];
            let count = categories.into_iter().filter(|&yes| yes).count();
            assert_eq!(count, 1, "{kind:?} is in {count} categories");
        }
    }

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
