mod char_cursor;
mod input_rejection;
mod lex_result;
mod lexical_error;

use char_cursor::CharCursor;

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
    Ok(CharCursor::new(text).run())
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

pub(crate) fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("lex rejects text longer than u32::MAX bytes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TokenKind;

    use LexicalErrorKind::{UnexpectedCharacter, UnterminatedString};
    use TokenKind::{Error, Newline, Whitespace};

    pub(super) fn check_lexer(text: &str, tokens: &[(TokenKind, &str)], errors: &[LexicalError]) {
        let result = lex(text).expect("the text is accepted");
        let mut pos = 0;
        let mut actual = Vec::new();
        for token in &result.tokens {
            let len = token.len as usize;
            actual.push((token.kind, &text[pos..pos + len]));
            pos += len;
        }
        assert_eq!(actual, tokens, "tokens of {text:?}");
        assert_eq!(pos, text.len(), "the tokens of {text:?} do not cover it");
        assert_eq!(result.errors, errors, "errors of {text:?}");
    }

    pub(super) fn error(kind: LexicalErrorKind, offset: u32, len: u32) -> LexicalError {
        LexicalError { kind, offset, len }
    }

    #[test]
    fn empty_input_has_no_tokens() {
        check_lexer("", &[], &[]);
    }

    #[test]
    fn nul_rejects_the_whole_document() {
        assert_eq!(lex("a\0b"), Err(InputRejection::NulCharacter { offset: 1 }));
    }

    #[test]
    fn rejection_reports_the_first_nul() {
        assert_eq!(
            lex("ab\0\0"),
            Err(InputRejection::NulCharacter { offset: 2 })
        );
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn offsets_past_u32_max_are_too_large() {
        let max = usize::try_from(u32::MAX).unwrap();
        assert_eq!(to_offset(max), Ok(u32::MAX));
        assert_eq!(to_offset(max + 1), Err(InputRejection::TooLarge));
    }

    #[test]
    fn bytes_are_lexed_like_the_text_they_decode_to() {
        assert_eq!(lex_bytes("-é_".as_bytes()), lex("-é_"));
    }

    #[test]
    fn invalid_utf8_rejects_the_whole_document() {
        assert_eq!(
            lex_bytes(b"ab\xffc"),
            Err(InputRejection::InvalidUtf8 { offset: 2 })
        );
    }

    #[test]
    fn truncated_sequence_is_invalid_utf8() {
        assert_eq!(
            lex_bytes(b"a\xc3"),
            Err(InputRejection::InvalidUtf8 { offset: 1 })
        );
    }

    #[test]
    fn encoded_surrogate_is_invalid_utf8() {
        assert_eq!(
            lex_bytes(b"\xed\xa0\x80"),
            Err(InputRejection::InvalidUtf8 { offset: 0 })
        );
    }

    #[test]
    fn nul_in_bytes_rejects_the_whole_document() {
        assert_eq!(
            lex_bytes(b"a\0"),
            Err(InputRejection::NulCharacter { offset: 1 })
        );
    }

    #[test]
    fn invalid_utf8_is_reported_before_nul() {
        assert_eq!(
            lex_bytes(b"\0\xff"),
            Err(InputRejection::InvalidUtf8 { offset: 1 })
        );
    }

    #[test]
    fn errors_are_reported_in_source_order() {
        check_lexer(
            "- \"x",
            &[
                (Error, "-"),
                (Whitespace, " "),
                (Error, "\"x"),
                (Newline, ""),
            ],
            &[
                error(UnexpectedCharacter, 0, 1),
                error(UnterminatedString, 2, 2),
            ],
        );
    }

    #[test]
    fn every_error_token_has_one_error_spanning_it() {
        for text in ["-é_", "a \"b\nc", "\u{FEFF}\u{FEFF}@-x", "# ok\n\"\\"] {
            let result = lex(text).unwrap();
            let mut pos = 0;
            let mut spans = Vec::new();
            for token in &result.tokens {
                if token.kind == Error {
                    spans.push((pos, token.len));
                }
                pos += token.len;
            }
            let reported: Vec<(u32, u32)> = result
                .errors
                .iter()
                .map(|error| (error.offset, error.len))
                .collect();
            assert_eq!(reported, spans, "input: {text:?}");
        }
    }

    #[test]
    fn token_lengths_cover_the_input() {
        for text in [
            "",
            "-é_",
            "ballot @city \"City\"\n",
            "  # comment\r\n",
            "日本語",
            "\u{FEFF}oval 1",
            "  a   b \t  ",
            "contest @mayor \"Mayor\" single\n  campaign @bsantos\n",
            "a \"b \\\" c\" \"unterminated\n\"x\\",
        ] {
            let total: usize = lex(text)
                .unwrap()
                .tokens
                .iter()
                .map(|token| token.len as usize)
                .sum();
            assert_eq!(total, text.len(), "input: {text:?}");
        }
    }
}
