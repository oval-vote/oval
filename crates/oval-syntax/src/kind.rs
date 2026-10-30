//! Token and node kinds of the syntax tree.

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    // Tokens. Every byte of the source is in exactly one token.
    WHITESPACE,
    NEWLINE,
    HASHES,
    AT,
    IDENT,
    GT,
    SLASHES,
    COMMENT_TEXT,
    KEYWORD,
    COMMA,
    COLON,
    LPAREN,
    RPAREN,
    PLUS,
    MINUS,
    STAR,
    NUMBER,
    DOT,
    TEXT,

    // Nodes.
    ROOT,
    MAGIC,
    HEADER,
    SECTION,
    HEADING,
    NAME,
    DESCRIPTION,
    DESC_LINE,
    KIND_LINE,
    TAG,
    FIELD,
    KEY,
    VALUE,
    CONTINUATION,
    CANDIDATE,
    MARKER,
    LABELS,
    LABEL,
    DESIGNATION,
    PARAGRAPH,
    PROSE_LINE,
    COMMENT,
    BLANK,
    ERROR,

    #[doc(hidden)]
    __LAST,
}

impl SyntaxKind {
    pub fn is_token(self) -> bool {
        self < SyntaxKind::ROOT
    }
    /// Lines that carry no meaning for the data model.
    pub fn is_trivia(self) -> bool {
        matches!(self, SyntaxKind::COMMENT | SyntaxKind::BLANK)
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {}

impl rowan::Language for Lang {
    type Kind = SyntaxKind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        assert!(raw.0 < SyntaxKind::__LAST as u16);
        // SAFETY: the enum is repr(u16), contiguous, and checked above.
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }
    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        kind.into()
    }
}
