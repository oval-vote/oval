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
    /// Every kind in declaration order, so `ALL[k as usize] == k`. This is
    /// what maps rowan's raw kinds back with a plain index.
    pub const ALL: [SyntaxKind; SyntaxKind::__LAST as usize] = [
        SyntaxKind::WHITESPACE,
        SyntaxKind::NEWLINE,
        SyntaxKind::HASHES,
        SyntaxKind::AT,
        SyntaxKind::IDENT,
        SyntaxKind::GT,
        SyntaxKind::SLASHES,
        SyntaxKind::COMMENT_TEXT,
        SyntaxKind::KEYWORD,
        SyntaxKind::COMMA,
        SyntaxKind::COLON,
        SyntaxKind::LPAREN,
        SyntaxKind::RPAREN,
        SyntaxKind::PLUS,
        SyntaxKind::MINUS,
        SyntaxKind::STAR,
        SyntaxKind::NUMBER,
        SyntaxKind::DOT,
        SyntaxKind::TEXT,
        SyntaxKind::ROOT,
        SyntaxKind::MAGIC,
        SyntaxKind::HEADER,
        SyntaxKind::SECTION,
        SyntaxKind::HEADING,
        SyntaxKind::NAME,
        SyntaxKind::DESCRIPTION,
        SyntaxKind::DESC_LINE,
        SyntaxKind::KIND_LINE,
        SyntaxKind::TAG,
        SyntaxKind::FIELD,
        SyntaxKind::KEY,
        SyntaxKind::VALUE,
        SyntaxKind::CONTINUATION,
        SyntaxKind::CANDIDATE,
        SyntaxKind::MARKER,
        SyntaxKind::LABELS,
        SyntaxKind::LABEL,
        SyntaxKind::DESIGNATION,
        SyntaxKind::PARAGRAPH,
        SyntaxKind::PROSE_LINE,
        SyntaxKind::COMMENT,
        SyntaxKind::BLANK,
        SyntaxKind::ERROR,
    ];

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
        SyntaxKind::ALL[raw.0 as usize]
    }
    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        kind.into()
    }
}

#[cfg(test)]
mod tests {
    use super::SyntaxKind;

    #[test]
    fn all_is_in_declaration_order() {
        for (i, k) in SyntaxKind::ALL.iter().enumerate() {
            assert_eq!(*k as usize, i, "{k:?} is out of place in ALL");
        }
        assert_eq!(SyntaxKind::ALL.len(), SyntaxKind::__LAST as usize);
    }
}
