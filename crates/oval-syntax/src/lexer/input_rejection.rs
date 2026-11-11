use std::fmt;

/// Why a document was rejected as a whole, before any of it was lexed.
///
/// [`lex`](crate::lex) and [`lex_bytes`](crate::lex_bytes) return a rejection, and no tokens, for
/// input that cannot be an Oval document at all. Any other document lexes completely, with its
/// flaws reported as [`LexicalError`](crate::LexicalError)s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputRejection {
    /// The document is longer than [`u32::MAX`] bytes.
    ///
    /// Offsets and lengths are `u32`, so this is the largest document that can be lexed.
    TooLarge,
    /// The document contains U+0000, which the specification does not allow anywhere in a document.
    NulCharacter {
        /// The byte offset of the first U+0000.
        offset: u32,
    },
    /// The document is not well-formed UTF-8.
    ///
    /// Only [`lex_bytes`](crate::lex_bytes) returns this, since a `&str` is always valid UTF-8.
    InvalidUtf8 {
        /// The byte offset of the first byte that does not begin a valid UTF-8 sequence.
        offset: u32,
    },
}

impl fmt::Display for InputRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => {
                write!(f, "the document is too large")
            }
            Self::NulCharacter { offset } => {
                write!(f, "the document contains U+0000 at byte {offset}")
            }
            Self::InvalidUtf8 { offset } => {
                write!(f, "the document is not valid UTF-8 at byte {offset}")
            }
        }
    }
}

impl std::error::Error for InputRejection {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nul_character_describes_itself() {
        let rejection = InputRejection::NulCharacter { offset: 7 };
        assert_eq!(
            rejection.to_string(),
            "the document contains U+0000 at byte 7"
        );
    }

    #[test]
    fn too_large_describes_itself() {
        assert_eq!(
            InputRejection::TooLarge.to_string(),
            "the document is too large"
        );
    }

    #[test]
    fn invalid_utf8_describes_itself() {
        let rejection = InputRejection::InvalidUtf8 { offset: 3 };
        assert_eq!(
            rejection.to_string(),
            "the document is not valid UTF-8 at byte 3"
        );
    }

    #[test]
    fn a_rejection_is_a_standard_error() {
        // Compiles only if `InputRejection` implements `std::error::Error`, so callers can use `?`
        // with other error types and box it.
        let error: Box<dyn std::error::Error> = Box::new(InputRejection::TooLarge);
        assert_eq!(error.to_string(), "the document is too large");
    }
}
