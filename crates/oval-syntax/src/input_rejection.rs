use std::fmt;

/// Why a document was rejected as a whole, before any of it was lexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputRejection {
    /// The document is longer than [`u32::MAX`] bytes, the largest this implementation accepts.
    TooLarge,
    /// The document contains U+0000, which is not permitted anywhere in a document.
    NulCharacter {
        /// The byte offset of the first U+0000.
        offset: u32,
    },
    /// The document is not well-formed UTF-8.
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
