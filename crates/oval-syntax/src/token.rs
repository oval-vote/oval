/// A token in an Oval document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The length of the token in bytes
    pub len: u32,
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
    pub(crate) fn is_ignored(self) -> bool {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_comments_blank_lines_and_byte_order_marks_are_ignored() {
        for kind in [
            TokenKind::Whitespace,
            TokenKind::Comment,
            TokenKind::BlankLine,
            TokenKind::ByteOrderMark,
        ] {
            assert!(kind.is_ignored(), "{kind:?} should be ignored");
        }
    }

    #[test]
    fn every_other_kind_is_not_ignored() {
        for kind in [
            TokenKind::Word,
            TokenKind::String,
            TokenKind::Integer,
            TokenKind::Date,
            TokenKind::AtSign,
            TokenKind::Url,
            TokenKind::Colon,
            TokenKind::Comma,
            TokenKind::MarkdownBlock,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::Dedent,
            TokenKind::Error,
        ] {
            assert!(!kind.is_ignored(), "{kind:?} should not be ignored");
        }
    }
}
