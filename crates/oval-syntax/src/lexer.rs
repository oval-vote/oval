mod cursor;
mod input_rejection;
mod lex_result;
mod lexical_error;

use cursor::Cursor;

pub use input_rejection::InputRejection;
pub use lex_result::LexResult;
pub use lexical_error::{LexicalError, LexicalErrorKind};

/// Splits an Oval document into tokens.
///
/// Every byte of `text` belongs to exactly one token, so the tokens reproduce `text` exactly and
/// their lengths add up to `text.len()`. See [`TokenKind`](crate::TokenKind) for the text each
/// kind of token covers.
///
/// Lexing never stops at an error. Text that cannot be lexed becomes a
/// [`TokenKind::Error`](crate::TokenKind::Error) token, with a [`LexicalError`] in
/// [`LexResult::errors`] that spans it, and lexing continues after it as the specification
/// prescribes. A few flaws, such as an invalid escape in a string, are reported without changing
/// the token they are in.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `text` contains U+0000, which the specification rejects as a
/// whole, or is longer than [`u32::MAX`] bytes. No tokens are produced.
///
/// # Examples
///
/// A token has a kind and a length but no text of its own. Its text is the slice of `text` that
/// begins where the token before it ends.
///
/// ```
/// use oval_syntax::{TokenKind, lex};
///
/// let text = "date 2026-11-03\n";
/// let result = lex(text)?;
/// let mut start = 0;
/// let mut tokens = Vec::new();
/// for token in &result.tokens {
///     let end = start + token.len as usize;
///     tokens.push((token.kind, &text[start..end]));
///     start = end;
/// }
/// assert_eq!(
///     tokens,
///     [
///         (TokenKind::Word, "date"),
///         (TokenKind::Whitespace, " "),
///         (TokenKind::Date, "2026-11-03"),
///         (TokenKind::Newline, "\n"),
///     ]
/// );
/// assert!(result.errors.is_empty());
/// # Ok::<(), oval_syntax::InputRejection>(())
/// ```
pub fn lex(text: &str) -> Result<LexResult, InputRejection> {
    to_offset(text.len())?;
    if let Some(offset) = text.find('\0') {
        return Err(InputRejection::NulCharacter {
            offset: to_offset(offset)?,
        });
    }
    Ok(Cursor::new(text).run())
}

/// Splits an Oval document given as raw bytes into tokens.
///
/// The bytes are decoded as UTF-8 and then lexed by [`lex`]. Use this where a document arrives as
/// bytes, such as when reading a file, so that malformed UTF-8 is rejected rather than repaired.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `bytes` is not well-formed UTF-8, contains U+0000, or is longer
/// than [`u32::MAX`] bytes. No tokens are produced.
///
/// # Examples
///
/// ```
/// use oval_syntax::{InputRejection, lex_bytes};
///
/// assert!(lex_bytes(b"date 2026-11-03\n").is_ok());
/// assert_eq!(
///     lex_bytes(b"note \xFF"),
///     Err(InputRejection::InvalidUtf8 { offset: 5 })
/// );
/// ```
pub fn lex_bytes(bytes: &[u8]) -> Result<LexResult, InputRejection> {
    to_offset(bytes.len())?;
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            return Err(InputRejection::InvalidUtf8 {
                offset: to_offset(error.valid_up_to())?,
            });
        }
    };
    lex(text)
}

fn to_offset(n: usize) -> Result<u32, InputRejection> {
    u32::try_from(n).map_err(|_| InputRejection::TooLarge)
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("lex rejects text longer than u32::MAX bytes")
}

#[cfg(test)]
mod tests;
