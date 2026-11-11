use crate::InputRejection;
use crate::{Token, TokenKind};

const BYTE_ORDER_MARK: char = '\u{FEFF}';

/// Splits `text` into tokens.
///
/// Every byte of `text` belongs to exactly one token, so the lengths of the returned tokens always
/// add up to `text.len()`.
///
/// So far, only a few tokens are recognized. A byte order mark (U+FEFF) at the very start of `text`
/// becomes a [`TokenKind::ByteOrderMark`] token, and each run of spaces becomes a
/// [`TokenKind::Whitespace`] token. Every other character becomes its own [`TokenKind::Error`]
/// token, which is how the specification treats a character that cannot begin any token.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `text` contains U+0000, which the specification rejects as a
/// whole, or is longer than [`u32::MAX`] bytes. No tokens are produced.
pub fn lex(text: &str) -> Result<Vec<Token>, InputRejection> {
    to_offset(text.len())?;
    if let Some(offset) = text.find('\0') {
        return Err(InputRejection::NulCharacter {
            offset: to_offset(offset)?,
        });
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
/// Returns an [`InputRejection`] if `bytes` is not well-formed UTF-8 or contains U+0000, which the
/// specification rejects as a whole, or is longer than [`u32::MAX`] bytes. No tokens are produced.
pub fn lex_bytes(bytes: &[u8]) -> Result<Vec<Token>, InputRejection> {
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
            if c == ' ' {
                let len = self.eat_while(|c| c == ' ');
                self.push(TokenKind::Whitespace, len);
            } else {
                self.bump(c);
                self.push(TokenKind::Error, c.len_utf8());
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn bump(&mut self, c: char) {
        self.pos += c.len_utf8();
    }

    fn eat_while(&mut self, predicate: impl Fn(char) -> bool) -> usize {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if !predicate(c) {
                break;
            }
            self.bump(c);
        }
        self.pos - start
    }

    fn push(&mut self, kind: TokenKind, len: usize) {
        // Cannot fail: `lex` rejects any text longer than u32::MAX bytes.
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

    // Offsets past u32::MAX only exist where usize is wider than 32 bits.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn offsets_past_u32_max_are_too_large() {
        let max = usize::try_from(u32::MAX).unwrap();
        assert_eq!(to_offset(max), Ok(u32::MAX));
        assert_eq!(to_offset(max + 1), Err(InputRejection::TooLarge));
    }

    #[test]
    fn too_large_describes_itself() {
        assert_eq!(
            InputRejection::TooLarge.to_string(),
            "the document is too large"
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
    fn a_run_of_spaces_is_one_whitespace_token() {
        assert_eq!(lex("   "), Ok(vec![token(TokenKind::Whitespace, 3)]));
    }

    #[test]
    fn whitespace_sits_between_other_tokens() {
        assert_eq!(
            lex("-  -"),
            Ok(vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Whitespace, 2),
                token(TokenKind::Error, 1),
            ])
        );
    }

    #[test]
    fn a_tab_is_not_whitespace() {
        assert_eq!(
            lex(" \t "),
            Ok(vec![
                token(TokenKind::Whitespace, 1),
                token(TokenKind::Error, 1),
                token(TokenKind::Whitespace, 1),
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
            "  a   b \t  ",
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
