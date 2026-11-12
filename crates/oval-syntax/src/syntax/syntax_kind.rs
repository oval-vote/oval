use crate::TokenKind;

/// The kind of a node or token in an Oval syntax tree.
///
/// The first variants match [`TokenKind`] one for one, in the same order, since
/// every token in the tree keeps the kind the lexer gave it. The variants after them are kinds of
/// node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    /// A [`TokenKind::Word`] token.
    Word,
    /// A [`TokenKind::String`] token.
    String,
    /// A [`TokenKind::Integer`] token.
    Integer,
    /// A [`TokenKind::Date`] token.
    Date,
    /// A [`TokenKind::AtSign`] token.
    AtSign,
    /// A [`TokenKind::Url`] token.
    Url,
    /// A [`TokenKind::Colon`] token.
    Colon,
    /// A [`TokenKind::Comma`] token.
    Comma,
    /// A [`TokenKind::MarkdownBlock`] token.
    MarkdownBlock,
    /// A [`TokenKind::Whitespace`] token.
    Whitespace,
    /// A [`TokenKind::Comment`] token.
    Comment,
    /// A [`TokenKind::ByteOrderMark`] token.
    ByteOrderMark,
    /// A [`TokenKind::BlankLine`] token.
    BlankLine,
    /// A [`TokenKind::Newline`] token.
    Newline,
    /// A [`TokenKind::Indent`] token.
    Indent,
    /// A [`TokenKind::Dedent`] token.
    Dedent,
    /// A [`TokenKind::Error`] token.
    Error,
    /// The root node of a syntax tree, which covers the whole document.
    Document,
    /// The version marker.
    VersionMarker,
    /// A field whose value is a string, such as `guide "Franklin Voter Guide"`.
    StringField,
    /// A field whose value is a date, such as `date 2026-11-03`.
    DateField,
    /// Text the parser discarded or set aside while recovering from a syntax error.
    ///
    /// The text is kept in the tree, so the tree still covers the whole document, but it contributes
    /// nothing to the node that contains it.
    ErrorNode,
}

impl SyntaxKind {
    pub(super) const ALL: [Self; SyntaxKind::ErrorNode as usize + 1] = [
        Self::Word,
        Self::String,
        Self::Integer,
        Self::Date,
        Self::AtSign,
        Self::Url,
        Self::Colon,
        Self::Comma,
        Self::MarkdownBlock,
        Self::Whitespace,
        Self::Comment,
        Self::ByteOrderMark,
        Self::BlankLine,
        Self::Newline,
        Self::Indent,
        Self::Dedent,
        Self::Error,
        Self::Document,
        Self::VersionMarker,
        Self::StringField,
        Self::DateField,
        Self::ErrorNode,
    ];
}

const _: () = {
    let mut i = 0;
    while i < SyntaxKind::ALL.len() {
        assert!(
            SyntaxKind::ALL[i] as usize == i,
            "SyntaxKind::ALL lists every kind in order"
        );
        i += 1;
    }
};

impl From<TokenKind> for SyntaxKind {
    fn from(value: TokenKind) -> Self {
        match value {
            TokenKind::Word => Self::Word,
            TokenKind::String => Self::String,
            TokenKind::Integer => Self::Integer,
            TokenKind::Date => Self::Date,
            TokenKind::AtSign => Self::AtSign,
            TokenKind::Url => Self::Url,
            TokenKind::Colon => Self::Colon,
            TokenKind::Comma => Self::Comma,
            TokenKind::MarkdownBlock => Self::MarkdownBlock,
            TokenKind::Whitespace => Self::Whitespace,
            TokenKind::Comment => Self::Comment,
            TokenKind::ByteOrderMark => Self::ByteOrderMark,
            TokenKind::BlankLine => Self::BlankLine,
            TokenKind::Newline => Self::Newline,
            TokenKind::Indent => Self::Indent,
            TokenKind::Dedent => Self::Dedent,
            TokenKind::Error => Self::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_kind_becomes_the_syntax_kind_of_the_same_name() {
        for (token_kind, syntax_kind) in [
            (TokenKind::Word, SyntaxKind::Word),
            (TokenKind::String, SyntaxKind::String),
            (TokenKind::Colon, SyntaxKind::Colon),
            (TokenKind::MarkdownBlock, SyntaxKind::MarkdownBlock),
            (TokenKind::Whitespace, SyntaxKind::Whitespace),
            (TokenKind::Newline, SyntaxKind::Newline),
            (TokenKind::Error, SyntaxKind::Error),
        ] {
            assert_eq!(SyntaxKind::from(token_kind), syntax_kind, "{token_kind:?}");
        }
    }

    #[test]
    fn into_converts_a_token_kind_too() {
        let kind: SyntaxKind = TokenKind::Dedent.into();
        assert_eq!(kind, SyntaxKind::Dedent);
    }
}
