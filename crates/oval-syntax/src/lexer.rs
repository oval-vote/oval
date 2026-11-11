use crate::InputRejection;
use crate::{Token, TokenKind};

const BYTE_ORDER_MARK: char = '\u{FEFF}';

/// Splits `text` into tokens.
///
/// Every byte of `text` belongs to exactly one token, so the lengths of the returned tokens always
/// add up to `text.len()`.
///
/// So far, only a few tokens are recognized. A byte order mark (U+FEFF) at the very start of `text`
/// becomes a [`TokenKind::ByteOrderMark`] token, each run of spaces becomes a
/// [`TokenKind::Whitespace`] token, and each `#` up to the end of its line becomes a
/// [`TokenKind::Comment`] token. Each line terminator, which is a line feed (U+000A), a carriage
/// return (U+000D) followed by a line feed, or a carriage return alone, becomes one
/// [`TokenKind::Newline`] token when its line has other tokens, and one [`TokenKind::BlankLine`]
/// token when the line contains only whitespace, a comment, or a byte order mark. Every other
/// character becomes its own
/// [`TokenKind::Error`] token, which is how the specification treats a character that cannot begin
/// any token.
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
        line_has_tokens: false,
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
    line_has_tokens: bool,
}

impl Lexer<'_> {
    fn run(&mut self) {
        if self.peek() == Some(BYTE_ORDER_MARK) {
            self.bump(BYTE_ORDER_MARK);
            self.push(TokenKind::ByteOrderMark, BYTE_ORDER_MARK.len_utf8());
        }
        while let Some(c) = self.peek() {
            match c {
                ' ' => self.whitespace(),
                '#' => self.comment(),
                '\n' | '\r' => self.line_terminator(c),
                '@' => self.at_sign(),
                ':' => self.colon(),
                ',' => self.comma(),
                _ => {
                    self.bump(c);
                    self.push(TokenKind::Error, c.len_utf8());
                }
            }
        }
    }

    fn whitespace(&mut self) {
        let len = self.eat_while(|c| c == ' ');
        self.push(TokenKind::Whitespace, len);
    }

    fn comment(&mut self) {
        let len = self.eat_while(|c| c != '\n' && c != '\r');
        self.push(TokenKind::Comment, len);
    }

    fn line_terminator(&mut self, c: char) {
        let start = self.pos;
        self.bump(c);
        if c == '\r' && self.peek() == Some('\n') {
            self.bump('\n');
        }
        let kind = if self.line_has_tokens {
            TokenKind::Newline
        } else {
            TokenKind::BlankLine
        };
        self.push(kind, self.pos - start);
        self.line_has_tokens = false;
    }

    fn at_sign(&mut self) {
        self.single(TokenKind::AtSign, '@');
    }

    fn colon(&mut self) {
        self.single(TokenKind::Colon, ':');
    }

    fn comma(&mut self) {
        self.single(TokenKind::Comma, ',');
    }

    fn single(&mut self, kind: TokenKind, c: char) {
        self.bump(c);
        self.push(kind, c.len_utf8());
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
        if !kind.is_ignored() {
            self.line_has_tokens = true;
        }
        let len = u32::try_from(len).expect("a token is at most u32::MAX bytes"); // Should be unreachable, validated ahead of time.
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
    fn invalid_utf8_is_reported_before_nul() {
        // Decoding comes first, so a later invalid byte wins over an earlier U+0000.
        assert_eq!(
            lex_bytes(b"\0\xff"),
            Err(InputRejection::InvalidUtf8 { offset: 1 })
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
    fn a_comment_runs_to_the_end_of_the_line() {
        assert_eq!(lex("# a note"), Ok(vec![token(TokenKind::Comment, 8)]));
    }

    #[test]
    fn a_comment_stops_before_a_line_terminator() {
        for text in ["# a\nb", "# a\rb", "# a\r\nb"] {
            assert_eq!(
                lex(text).unwrap()[0],
                token(TokenKind::Comment, 3),
                "input: {text:?}"
            );
        }
    }

    #[test]
    fn a_lone_hash_is_a_comment() {
        assert_eq!(lex("#"), Ok(vec![token(TokenKind::Comment, 1)]));
    }

    #[test]
    fn a_comment_may_follow_other_tokens() {
        assert_eq!(
            lex("- # x"),
            Ok(vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Whitespace, 1),
                token(TokenKind::Comment, 3),
            ])
        );
    }

    #[test]
    fn a_comment_may_contain_anything_but_a_line_terminator() {
        assert_eq!(lex("#\t@\"é#"), Ok(vec![token(TokenKind::Comment, 7)]));
    }

    #[test]
    fn a_line_feed_after_tokens_is_a_newline() {
        assert_eq!(
            lex("-\n"),
            Ok(vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Newline, 1)
            ])
        );
    }

    #[test]
    fn a_line_feed_on_an_empty_line_is_a_blank_line() {
        assert_eq!(lex("\n"), Ok(vec![token(TokenKind::BlankLine, 1)]));
    }

    #[test]
    fn a_line_of_whitespace_and_a_comment_is_a_blank_line() {
        assert_eq!(
            lex("  # x\n"),
            Ok(vec![
                token(TokenKind::Whitespace, 2),
                token(TokenKind::Comment, 3),
                token(TokenKind::BlankLine, 1),
            ])
        );
    }

    #[test]
    fn a_line_with_only_a_byte_order_mark_is_a_blank_line() {
        assert_eq!(
            lex("\u{FEFF}\n"),
            Ok(vec![
                token(TokenKind::ByteOrderMark, 3),
                token(TokenKind::BlankLine, 1),
            ])
        );
    }

    #[test]
    fn a_comment_after_tokens_keeps_the_newline() {
        assert_eq!(
            lex("- # x\n"),
            Ok(vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Whitespace, 1),
                token(TokenKind::Comment, 3),
                token(TokenKind::Newline, 1),
            ])
        );
    }

    #[test]
    fn each_line_is_judged_on_its_own_tokens() {
        assert_eq!(
            lex("-\n\n-\n"),
            Ok(vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Newline, 1),
                token(TokenKind::BlankLine, 1),
                token(TokenKind::Error, 1),
                token(TokenKind::Newline, 1),
            ])
        );
    }

    #[test]
    fn every_line_terminator_ends_a_line() {
        for (terminator, len) in [("\n", 1), ("\r\n", 2), ("\r", 1)] {
            assert_eq!(
                lex(&format!("-{terminator}{terminator}")),
                Ok(vec![
                    token(TokenKind::Error, 1),
                    token(TokenKind::Newline, len),
                    token(TokenKind::BlankLine, len),
                ]),
                "terminator: {terminator:?}"
            );
        }
    }

    #[test]
    fn a_carriage_return_and_line_feed_are_one_terminator() {
        assert_eq!(lex("\r\n"), Ok(vec![token(TokenKind::BlankLine, 2)]));
    }

    #[test]
    fn a_line_feed_before_a_carriage_return_is_two_terminators() {
        assert_eq!(
            lex("\n\r"),
            Ok(vec![
                token(TokenKind::BlankLine, 1),
                token(TokenKind::BlankLine, 1),
            ])
        );
    }

    #[test]
    fn a_carriage_return_before_a_crlf_is_two_terminators() {
        assert_eq!(
            lex("-\r\r\n"),
            Ok(vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Newline, 1),
                token(TokenKind::BlankLine, 2),
            ])
        );
    }

    #[test]
    fn terminators_may_be_mixed_in_one_document() {
        assert_eq!(
            lex("-\r\n-\r-\n"),
            Ok(vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Newline, 2),
                token(TokenKind::Error, 1),
                token(TokenKind::Newline, 1),
                token(TokenKind::Error, 1),
                token(TokenKind::Newline, 1),
            ])
        );
    }

    #[test]
    fn a_comment_line_ending_in_crlf_is_a_blank_line() {
        assert_eq!(
            lex("# x\r\n"),
            Ok(vec![
                token(TokenKind::Comment, 3),
                token(TokenKind::BlankLine, 2),
            ])
        );
    }

    #[test]
    fn an_at_sign_is_its_own_token() {
        assert_eq!(lex("@"), Ok(vec![token(TokenKind::AtSign, 1)]));
    }

    #[test]
    fn a_colon_is_its_own_token() {
        assert_eq!(lex(":"), Ok(vec![token(TokenKind::Colon, 1)]));
    }

    #[test]
    fn a_comma_is_its_own_token() {
        assert_eq!(lex(","), Ok(vec![token(TokenKind::Comma, 1)]));
    }

    #[test]
    fn adjacent_punctuators_are_separate_tokens() {
        assert_eq!(
            lex("@:,"),
            Ok(vec![
                token(TokenKind::AtSign, 1),
                token(TokenKind::Colon, 1),
                token(TokenKind::Comma, 1),
            ])
        );
    }

    #[test]
    fn a_line_with_a_punctuator_ends_in_a_newline() {
        for text in ["@\n", ":\n", ",\n"] {
            assert_eq!(
                lex(text).unwrap()[1],
                token(TokenKind::Newline, 1),
                "input: {text:?}"
            );
        }
    }

    #[test]
    fn punctuators_inside_a_comment_are_part_of_it() {
        assert_eq!(lex("# @:,"), Ok(vec![token(TokenKind::Comment, 5)]));
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
