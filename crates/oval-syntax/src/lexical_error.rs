use std::fmt;

/// The kinds of lexical errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LexicalErrorKind {
    /// An unexpected character
    UnexpectedCharacter,
    /// A string with no closing quote before the end of its line
    UnterminatedString,
    /// A backslash in a string followed by anything other than `"` or `\`
    InvalidEscape,
}

impl LexicalErrorKind {
    /// The identifier of the error, as described in the specification, such as `invalid-escape`
    pub fn spec_id(self) -> &'static str {
        match self {
            Self::UnexpectedCharacter => "unexpected-character",
            Self::UnterminatedString => "unterminated-string",
            Self::InvalidEscape => "invalid-escape",
        }
    }
}

/// A lexical error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LexicalError {
    /// The kind of error
    pub kind: LexicalErrorKind,
    /// The offset, in bytes, in the document where the error occurred
    pub offset: u32,
    /// How many bytes the error spans
    pub len: u32,
}

impl LexicalError {
    /// The identifier of the error, as described in the specification
    pub fn spec_id(self) -> &'static str {
        self.kind.spec_id()
    }
}

impl fmt::Display for LexicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.kind.spec_id(), self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_its_specification_identifier() {
        for (kind, identifier) in [
            (
                LexicalErrorKind::UnexpectedCharacter,
                "unexpected-character",
            ),
            (LexicalErrorKind::UnterminatedString, "unterminated-string"),
            (LexicalErrorKind::InvalidEscape, "invalid-escape"),
        ] {
            assert_eq!(kind.spec_id(), identifier, "{kind:?}");
        }
    }

    #[test]
    fn an_error_has_the_identifier_of_its_kind() {
        let error = LexicalError {
            kind: LexicalErrorKind::InvalidEscape,
            offset: 9,
            len: 2,
        };
        assert_eq!(error.spec_id(), "invalid-escape");
    }

    #[test]
    fn an_error_describes_its_identifier_and_offset() {
        let error = LexicalError {
            kind: LexicalErrorKind::UnterminatedString,
            offset: 12,
            len: 4,
        };
        assert_eq!(error.to_string(), "unterminated-string at byte 12");
    }
}
