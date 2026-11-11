use std::fmt;

use crate::{Token, TokenKind};

const BYTE_ORDER_MARK: char = '\u{FEFF}';

/// Splits `text` into tokens.
///
/// Every byte of `text` belongs to exactly one token, so the lengths of the returned tokens always
/// add up to `text.len()`. Nothing is recognized yet: each character becomes its own
/// [`TokenKind::Error`] token, which is how the specification treats a character that cannot begin
/// any token. The one exception is a byte order mark (U+FEFF) at the very start of `text`, which
/// becomes a [`TokenKind::ByteOrderMark`] token; anywhere else, U+FEFF is an error like any other
/// unexpected character.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `text` contains U+0000. The specification rejects such a
/// document as a whole, so no tokens are produced.
///
/// # Panics
///
/// Panics if `text` is longer than [`u32::MAX`] bytes.
pub fn lex(text: &str) -> Result<Vec<Token>, InputRejection> {
    if let Some(offset) = text.find('\0') {
        let offset = u32::try_from(offset).expect("a document is at most u32::MAX bytes");
        return Err(InputRejection::NulCharacter { offset });
    }
    let mut lexer = Lexer {
        source: text,
        pos: 0,
        tokens: Vec::new(),
    };
    lexer.run();
    Ok(lexer.tokens)
}

/// Splits a document given as raw bytes into tokens.
///
/// The bytes are decoded as UTF-8 and then lexed by [`lex`]. Use this where a document arrives as
/// bytes, such as when reading a file, so that malformed UTF-8 is rejected rather than repaired.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `bytes` is not well-formed UTF-8 or contains U+0000. The
/// specification rejects such a document as a whole, so no tokens are produced.
///
/// # Panics
///
/// Panics if `bytes` is longer than [`u32::MAX`] bytes.
pub fn lex_bytes(bytes: &[u8]) -> Result<Vec<Token>, InputRejection> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        let offset =
            u32::try_from(error.valid_up_to()).expect("a document is at most u32::MAX bytes");
        InputRejection::InvalidUtf8 { offset }
    })?;
    lex(text)
}

/// Why a document was rejected as a whole, before any of it was lexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputRejection {
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

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn run(&mut self) {
        if self.peek() == Some(BYTE_ORDER_MARK) {
            self.bump(BYTE_ORDER_MARK);
            self.push(TokenKind::ByteOrderMark, BYTE_ORDER_MARK.len_utf8());
        }
        while let Some(c) = self.peek() {
            self.bump(c);
            self.push(TokenKind::Error, c.len_utf8());
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn bump(&mut self, c: char) {
        self.pos += c.len_utf8();
    }

    fn push(&mut self, kind: TokenKind, len: usize) {
        let len = u32::try_from(len).expect("a token is at most u32::MAX bytes");
        self.tokens.push(Token { kind, len });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(kind: TokenKind, len: u32) -> Token {
        Token { kind, len }
    }

    #[test]
    fn empty_input_has_no_tokens() {
        assert_eq!(lex(""), Ok(vec![]));
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

    #[test]
    fn rejection_describes_itself() {
        let rejection = InputRejection::NulCharacter { offset: 7 };
        assert_eq!(
            rejection.to_string(),
            "the document contains U+0000 at byte 7"
        );
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
        // The first byte of "é" without its second byte.
        assert_eq!(
            lex_bytes(b"a\xc3"),
            Err(InputRejection::InvalidUtf8 { offset: 1 })
        );
    }

    #[test]
    fn encoded_surrogate_is_invalid_utf8() {
        // U+D800 encoded as if it were a character. Well-formed UTF-8 never contains surrogates.
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
    fn invalid_utf8_describes_itself() {
        let rejection = InputRejection::InvalidUtf8 { offset: 3 };
        assert_eq!(
            rejection.to_string(),
            "the document is not valid UTF-8 at byte 3"
        );
    }

    #[test]
    fn each_unexpected_character_is_its_own_error_token() {
        // "é" is one character but two bytes in UTF-8.
        assert_eq!(
            lex("-é_"),
            Ok(vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Error, 2),
                token(TokenKind::Error, 1),
            ])
        );
    }

    #[test]
    fn leading_byte_order_mark_is_its_own_token() {
        assert_eq!(
            lex("\u{FEFF}-"),
            Ok(vec![
                token(TokenKind::ByteOrderMark, 3),
                token(TokenKind::Error, 1),
            ])
        );
    }

    #[test]
    fn byte_order_mark_elsewhere_is_an_error() {
        assert_eq!(
            lex("-\u{FEFF}"),
            Ok(vec![token(TokenKind::Error, 1), token(TokenKind::Error, 3)])
        );
    }

    #[test]
    fn only_the_first_byte_order_mark_is_ignored() {
        assert_eq!(
            lex("\u{FEFF}\u{FEFF}"),
            Ok(vec![
                token(TokenKind::ByteOrderMark, 3),
                token(TokenKind::Error, 3),
            ])
        );
    }

    #[test]
    fn byte_order_mark_survives_decoding() {
        assert_eq!(
            lex_bytes(b"\xef\xbb\xbf-"),
            Ok(vec![
                token(TokenKind::ByteOrderMark, 3),
                token(TokenKind::Error, 1),
            ])
        );
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
        ] {
            let total: usize = lex(text)
                .unwrap()
                .iter()
                .map(|token| token.len as usize)
                .sum();
            assert_eq!(total, text.len(), "input: {text:?}");
        }
    }
}
