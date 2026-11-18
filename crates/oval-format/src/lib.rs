//! Formatter for the Oval voting guide format.
//!
//! [`format()`] rewrites a well-formed document in the canonical style, with two spaces of
//! indentation for each enclosing block, one space between the tokens of a line, and `\n` at the
//! end of every line.
//!
//! ```
//! let text = "oval 1\nballot   @city \"City\"\n    measure @a \"A\"\n        recommend yes\n";
//! let formatted = oval_format::format(text)?;
//! assert_eq!(
//!     formatted,
//!     "oval 1\nballot @city \"City\"\n  measure @a \"A\"\n    recommend yes\n"
//! );
//! # Ok::<(), oval_format::FormatError>(())
//! ```

mod entry;
mod methods;
mod modifiers;
mod order;
mod print;
mod references;

use std::fmt;

use oval_syntax::{InputRejection, ParseResult};

/// The reason [`format()`] can't format a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// The input was rejected before lexing, so nothing in it was read.
    Rejected(InputRejection),
    /// The document has lexical or syntax errors.
    ///
    /// A tree recovered from errors isn't the shape the author meant, so formatting it could move
    /// text in surprising ways. The parse is kept so a caller can report the errors without parsing
    /// again.
    Malformed(ParseResult),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(rejection) => rejection.fmt(f),
            Self::Malformed(_) => write!(f, "the document has lexical or syntax errors"),
        }
    }
}

impl std::error::Error for FormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Rejected(rejection) => Some(rejection),
            Self::Malformed(_) => None,
        }
    }
}

/// Formats a document in the canonical style.
///
/// Validation errors don't stop it, so an author can format a guide that isn't finished.
///
/// # Errors
///
/// Returns [`FormatError::Rejected`] if the input is rejected before lexing, and
/// [`FormatError::Malformed`] if the document has lexical or syntax errors.
///
/// ```
/// let result = oval_format::format("oval 1\nguide \"G\n");
/// assert!(matches!(result, Err(oval_format::FormatError::Malformed(_))));
/// ```
pub fn format(text: &str) -> Result<String, FormatError> {
    let parse = oval_syntax::parse(text).map_err(FormatError::Rejected)?;
    if !parse.lexical_errors.is_empty() || !parse.syntax_errors.is_empty() {
        return Err(FormatError::Malformed(parse));
    }
    Ok(print::print(&entry::document(&parse.syntax())))
}

#[cfg(test)]
mod tests {
    use oval_syntax::InputRejection;

    use super::{FormatError, format};

    #[test]
    fn input_rejected_before_lexing_is_not_formatted() {
        assert_eq!(
            format("oval 1\0\n"),
            Err(FormatError::Rejected(InputRejection::NulCharacter {
                offset: 6
            }))
        );
    }

    #[test]
    fn a_document_with_lexical_or_syntax_errors_is_not_formatted() {
        for text in [
            "oval 1\nguide \"G\n",
            "oval 1\nguide\n",
            "oval 1\n  guide \"G\"\n",
            "oval\t1\n",
        ] {
            let Err(FormatError::Malformed(parse)) = format(text) else {
                panic!("{text:?} is formatted");
            };
            assert!(
                !parse.lexical_errors.is_empty() || !parse.syntax_errors.is_empty(),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_document_with_validation_errors_is_formatted() {
        assert_eq!(format("oval  1\n"), Ok("oval 1\n".to_owned()));
    }
}
