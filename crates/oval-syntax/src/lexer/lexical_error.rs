use std::fmt;

/// The kind of a [`LexicalError`], one for each lexical error in Section 3.1 of the specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LexicalErrorKind {
    /// A character that cannot begin a token, such as `-`, `\`, or any character outside ASCII.
    ///
    /// The character becomes an [`Error`](crate::TokenKind::Error) token on its own.
    UnexpectedCharacter,
    /// A string with no closing quote before the end of its line.
    ///
    /// Everything from the opening quote to the end of the line becomes one
    /// [`Error`](crate::TokenKind::Error) token.
    UnterminatedString,
    /// A backslash in a string followed by anything other than `"` or `\`.
    ///
    /// The error spans the backslash and the character after it, and the string is still a
    /// [`String`](crate::TokenKind::String) token. It is not reported in an unterminated string,
    /// which is already an error as a whole.
    InvalidEscape,
    /// A run of tabs outside a string, a comment, or a Markdown body.
    ///
    /// The run becomes one [`Error`](crate::TokenKind::Error) token.
    IllegalTab,
    /// A run of letters, digits, `-`, `_`, and `.` that begins with a digit but is neither an
    /// integer nor a date, such as `12abc` or `2026-11-5`.
    ///
    /// The run becomes one [`Error`](crate::TokenKind::Error) token.
    MalformedNumber,
    /// `http://` or `https://` with nothing after it before a space, a tab, or the end of its line.
    ///
    /// The scheme and its `://` become one [`Error`](crate::TokenKind::Error) token.
    MalformedUrl,
    /// A line indented less deeply than its block, but not at the level of any enclosing block.
    ///
    /// The error spans the line's leading spaces, which are still a
    /// [`Whitespace`](crate::TokenKind::Whitespace) token, and the line is treated as being at the
    /// level of the innermost block it did not close.
    InconsistentDedent,
}

impl LexicalErrorKind {
    /// Returns this error's identifier in the specification, such as `"invalid-escape"`.
    ///
    /// Identifiers do not change between editions of the specification, so tools can use them to
    /// name an error or look it up.
    pub fn spec_id(self) -> &'static str {
        match self {
            Self::UnexpectedCharacter => "unexpected-character",
            Self::UnterminatedString => "unterminated-string",
            Self::InvalidEscape => "invalid-escape",
            Self::IllegalTab => "illegal-tab",
            Self::MalformedNumber => "malformed-number",
            Self::MalformedUrl => "malformed-url",
            Self::InconsistentDedent => "inconsistent-dedent",
        }
    }
}

/// A flaw found while lexing a document, and the text it spans.
///
/// Most errors span exactly one [`Error`](crate::TokenKind::Error) token. An
/// [`InvalidEscape`](LexicalErrorKind::InvalidEscape) or
/// [`InconsistentDedent`](LexicalErrorKind::InconsistentDedent) error instead spans part of a token
/// that keeps its kind.
///
/// Its [`Display`](fmt::Display) form names the error and where it begins, such as
/// `invalid-escape at byte 18`.
///
/// # Examples
///
/// ```
/// use oval_syntax::{LexicalErrorKind, lex};
///
/// let text = "subtitle \"Measure \\#3\"\n";
/// let error = lex(text)?.errors[0];
/// assert_eq!(error.kind, LexicalErrorKind::InvalidEscape);
///
/// let start = error.offset as usize;
/// assert_eq!(&text[start..start + error.len as usize], "\\#");
/// # Ok::<(), oval_syntax::InputRejection>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LexicalError {
    /// The kind of error.
    pub kind: LexicalErrorKind,
    /// The byte offset in the document where the error's span begins.
    pub offset: u32,
    /// The length of the error's span in bytes.
    pub len: u32,
}

impl LexicalError {
    /// Returns the identifier of this error's kind in the specification. See
    /// [`LexicalErrorKind::spec_id`].
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
            (LexicalErrorKind::IllegalTab, "illegal-tab"),
            (LexicalErrorKind::MalformedNumber, "malformed-number"),
            (LexicalErrorKind::MalformedUrl, "malformed-url"),
            (LexicalErrorKind::InconsistentDedent, "inconsistent-dedent"),
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
