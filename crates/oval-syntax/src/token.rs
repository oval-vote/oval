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

/// The kind of a [`Token`], which says what text it covers.
///
/// Every kind is in exactly one of four categories. Lexical tokens carry a document's content,
/// ignored tokens are skipped by the parser, layout tokens give a document its structure, and error
/// tokens cover text that cannot be lexed. See [`is_lexical`](Self::is_lexical),
/// [`is_ignored`](Self::is_ignored), [`is_layout`](Self::is_layout), and
/// [`is_error`](Self::is_error).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A run of ASCII letters, digits, `-`, and `_` that begins with a letter, such as `ballot` or
    /// `vote-for`.
    ///
    /// Keywords are words too. Which words are keywords, and where, is up to the parser.
    Word,
    /// Text in double quotes on one line, such as `"Vote for one"`.
    ///
    /// The token includes the quotes. Inside a string, `\"` is a quote and `\\` is a backslash. Any
    /// other escape is an [`InvalidEscape`](crate::LexicalErrorKind::InvalidEscape) error, but the
    /// string is still a `String` token. A string with no closing quote before the end of its line
    /// is an [`UnterminatedString`](crate::LexicalErrorKind::UnterminatedString) error token
    /// instead.
    String,
    /// A run of ASCII digits, such as `42`.
    ///
    /// Integers are unsigned, and leading zeros are allowed. A run that begins with a digit but is
    /// neither an integer nor a date, such as `12abc` or `1.5`, is a
    /// [`MalformedNumber`](crate::LexicalErrorKind::MalformedNumber) error token.
    Integer,
    /// A date in the shape `YYYY-MM-DD`, such as `2026-11-03`.
    ///
    /// Only the shape is checked, so `2026-02-30` is a `Date` too.
    Date,
    /// `@`, which begins an identifier such as `@dana-lombardi`.
    AtSign,
    /// A URL that begins with `http://` or `https://`, such as `https://example.com`.
    ///
    /// A URL runs up to the next space, tab, or line terminator. It is written without quotes, so a
    /// `#` or `"` inside one is part of it. A scheme with nothing after its `://` is a
    /// [`MalformedUrl`](crate::LexicalErrorKind::MalformedUrl) error token.
    Url,
    /// `:`, which opens a Markdown body when it is the last token on its line.
    Colon,
    /// `,`, which separates the strings in a list.
    Comma,
    /// The body of a Markdown field, such as the text under `note:`.
    ///
    /// A body follows a line whose last token, ignoring whitespace and comments, is a
    /// [`Colon`](Self::Colon). Blank lines after that line are [`BlankLine`](Self::BlankLine)
    /// tokens. If the next line with content is indented more deeply than the opening line, the
    /// body begins there; otherwise there is no body. The body runs over every following line that
    /// is blank or indented more deeply than the opening line, and ends with the terminator of its
    /// last line that is not blank. Blank lines after it are `BlankLine` tokens.
    ///
    /// The token covers the body's raw text, including each line's leading spaces. Nothing inside
    /// it is lexed, so `#`, `@`, `"`, and tabs there are just text, and it produces no
    /// [`Newline`](Self::Newline), [`Indent`](Self::Indent), or [`Dedent`](Self::Dedent) tokens.
    MarkdownBlock,
    /// A run of spaces (U+0020).
    ///
    /// Tabs are not whitespace. A run of tabs outside a string, a comment, or a Markdown body is an
    /// [`IllegalTab`](crate::LexicalErrorKind::IllegalTab) error token.
    Whitespace,
    /// A `#` and everything after it up to the end of its line.
    ///
    /// A `#` inside a string, a URL, or a Markdown body does not begin a comment.
    Comment,
    /// A byte order mark (U+FEFF) at the very start of a document.
    ///
    /// The first line's indentation is counted after it. U+FEFF anywhere else is an
    /// [`UnexpectedCharacter`](crate::LexicalErrorKind::UnexpectedCharacter) error token.
    ByteOrderMark,
    /// The line terminator of a blank line, which has nothing on it but whitespace, a comment, or a
    /// byte order mark.
    ///
    /// Blank lines do not end a line for the parser, and they do not affect indentation.
    BlankLine,
    /// The end of a line with tokens.
    ///
    /// It covers the line's terminator, which is a line feed (U+000A), a carriage return (U+000D)
    /// followed by a line feed, or a carriage return alone. If the last line has tokens but no
    /// terminator, its `Newline` has no text, so every line with tokens ends in a `Newline`.
    Newline,
    /// The start of a block, before the first token of a line indented more deeply than the
    /// enclosing block.
    ///
    /// A line's indentation is the number of spaces before its first other character, and any
    /// increase opens a block. An `Indent` has no text, and comes after the line's leading
    /// [`Whitespace`](Self::Whitespace).
    Indent,
    /// The end of a block, for each block a line leaves by being indented less deeply.
    ///
    /// A `Dedent` has no text, and comes after the line's leading [`Whitespace`](Self::Whitespace).
    /// Every block still open at the end of input is closed there. A line indented between two
    /// enclosing levels is an [`InconsistentDedent`](crate::LexicalErrorKind::InconsistentDedent)
    /// error.
    Dedent,
    /// Text that cannot be lexed.
    ///
    /// Each `Error` token has a [`LexicalError`](crate::LexicalError) that spans it exactly.
    Error,
}

impl TokenKind {
    /// Returns `true` if the parser ignores this kind of token.
    ///
    /// The ignored kinds are [`Whitespace`](Self::Whitespace), [`Comment`](Self::Comment),
    /// [`ByteOrderMark`](Self::ByteOrderMark), and [`BlankLine`](Self::BlankLine). Ignored tokens
    /// matter only for reproducing a document's text, so a formatter keeps them.
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

    /// Returns `true` if this kind of token gives a document its structure.
    ///
    /// The layout kinds are [`Newline`](Self::Newline), [`Indent`](Self::Indent), and
    /// [`Dedent`](Self::Dedent).
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

    /// Returns `true` if this kind of token carries a document's content.
    ///
    /// Every kind but the ignored, layout, and error kinds is lexical.
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

    /// Returns `true` if this is [`Error`](Self::Error).
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
