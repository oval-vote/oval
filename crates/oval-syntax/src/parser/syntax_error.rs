use std::fmt;

use crate::{Keyword, TokenKind};

/// The kind of a [`SyntaxError`], one for each syntax error in Section 3.2 of the specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxErrorKind {
    /// A line that ends where a value is required, such as after `oval` or after a field's keyword.
    ///
    /// The error has no length and sits at the end of the line. The parser assumes the value and
    /// keeps the line with nothing in its place.
    MissingValue,
    /// A token the grammar does not accept at that point on a line, such as a string after `date` or
    /// a second value after a complete field.
    ///
    /// The error spans from that token to the end of the line. The parser discards those tokens,
    /// keeps what it parsed before them, and continues on the next line.
    UnexpectedToken,
    /// A Markdown field's line that ends with `:` without an indented body below it.
    ///
    /// The error has no length and sits just after the line's terminator. The parser keeps the field
    /// with an empty body.
    MissingBody,
    /// A field whose keyword already appeared in the same block, or at the document level.
    ///
    /// The error spans the second field, including its body. The parser parses the field in full,
    /// so that errors inside it are still reported, but sets it aside, and the first one stands.
    RepeatedField,
    /// A line that ends, or has a string, where an identifier is required, or an `@` that is not
    /// followed by a word.
    ///
    /// The error has no length. It sits where the identifier would begin, or just after an `@` that
    /// has no word. The parser assumes the identifier and continues with the rest of the line.
    MissingIdentifier,
    /// A declaration whose identifier is followed by the end of the line instead of its name.
    ///
    /// The error has no length and sits at the end of the line. The parser keeps the declaration
    /// without a name.
    MissingName,
}

impl SyntaxErrorKind {
    /// Returns this error's identifier in the specification, such as `"missing-value"`.
    ///
    /// Identifiers do not change between editions of the specification, so tools can use them to
    /// name an error or look it up.
    pub fn spec_id(self) -> &'static str {
        match self {
            Self::MissingValue => "missing-value",
            Self::UnexpectedToken => "unexpected-token",
            Self::MissingBody => "missing-body",
            Self::RepeatedField => "repeated-field",
            Self::MissingIdentifier => "missing-identifier",
            Self::MissingName => "missing-name",
        }
    }
}

/// Something the grammar would have accepted where a [`SyntaxError`] occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Expected {
    /// A token of this kind, such as an [`Integer`](TokenKind::Integer).
    Token(TokenKind),
    /// A word that is this keyword, such as `guide`.
    Keyword(Keyword),
}

/// A flaw found while parsing a document, and the text it spans.
///
/// A missing token is reported as an error with no length, where the token would have begun. Its
/// [`Display`](fmt::Display) form names the error and where it begins, such as
/// `missing-value at byte 4`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyntaxError {
    /// The kind of error.
    pub kind: SyntaxErrorKind,
    /// The byte offset in the document where the error's span begins.
    pub offset: u32,
    /// The length of the error's span in bytes.
    pub len: u32,
    /// What the grammar would have accepted instead, so that a diagnostic can name it.
    pub expected: Vec<Expected>,
}

impl SyntaxError {
    /// Returns the identifier of this error's kind in the specification. See
    /// [`SyntaxErrorKind::spec_id`].
    pub fn spec_id(&self) -> &'static str {
        self.kind.spec_id()
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.kind.spec_id(), self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing_integer() -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingValue,
            offset: 4,
            len: 0,
            expected: vec![Expected::Token(TokenKind::Integer)],
        }
    }

    #[test]
    fn each_kind_has_its_specification_identifier() {
        assert_eq!(SyntaxErrorKind::MissingValue.spec_id(), "missing-value");
        assert_eq!(
            SyntaxErrorKind::UnexpectedToken.spec_id(),
            "unexpected-token"
        );
        assert_eq!(SyntaxErrorKind::MissingBody.spec_id(), "missing-body");
        assert_eq!(SyntaxErrorKind::RepeatedField.spec_id(), "repeated-field");
        assert_eq!(
            SyntaxErrorKind::MissingIdentifier.spec_id(),
            "missing-identifier"
        );
        assert_eq!(SyntaxErrorKind::MissingName.spec_id(), "missing-name");
    }

    #[test]
    fn an_error_has_the_identifier_of_its_kind() {
        assert_eq!(missing_integer().spec_id(), "missing-value");
    }

    #[test]
    fn an_error_describes_its_identifier_and_offset() {
        assert_eq!(missing_integer().to_string(), "missing-value at byte 4");
    }
}
