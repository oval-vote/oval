use crate::{InputRejection, LexResult, LexicalError, LexicalErrorKind, Token, TokenKind};

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
/// token when the line contains only whitespace, a comment, or a byte order mark. Each `@`, `:`,
/// and `,` becomes a [`TokenKind::AtSign`], [`TokenKind::Colon`], or [`TokenKind::Comma`] token, and
/// each ASCII letter followed by any ASCII letters, digits, `-`, and `_` becomes a
/// [`TokenKind::Word`] token. Each `"` up to its closing `"` becomes a [`TokenKind::String`]
/// token, where `\"` and `\\` do not close it; a string with no closing `"` before the end of its
/// line becomes a single [`TokenKind::Error`] token instead. Every other character becomes its own
/// [`TokenKind::Error`] token, which is how the specification treats a character that cannot begin
/// any token.
///
/// Lexing never stops at an error. Each [`TokenKind::Error`] token has a [`LexicalError`] in
/// [`LexResult::errors`] spanning exactly its bytes, and lexing continues after it.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `text` contains U+0000, which the specification rejects as a
/// whole, or is longer than [`u32::MAX`] bytes. No tokens are produced.
pub fn lex(text: &str) -> Result<LexResult, InputRejection> {
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
        errors: Vec::new(),
        line_has_tokens: false,
    };
    lexer.run();
    Ok(LexResult {
        tokens: lexer.tokens,
        errors: lexer.errors,
    })
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

// Cannot fail: `lex` rejects any text longer than u32::MAX bytes, so every position and length
// within it fits.
fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("lex rejects text longer than u32::MAX bytes")
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<LexicalError>,
    line_has_tokens: bool,
}

impl Lexer<'_> {
    fn run(&mut self) {
        if self.rest().starts_with(BYTE_ORDER_MARK) {
            let start = self.pos;
            self.bump(BYTE_ORDER_MARK);
            self.push(TokenKind::ByteOrderMark, start);
        }
        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                ' ' => self.whitespace(start),
                '#' => self.comment(start),
                '\n' | '\r' => self.line_terminator(c, start),
                '@' => self.at_sign(start),
                ':' => self.colon(start),
                ',' => self.comma(start),
                '"' => self.string(start),
                c if c.is_ascii_alphabetic() => self.word(start),
                _ => {
                    self.bump(c);
                    self.push_error(LexicalErrorKind::UnexpectedCharacter, start);
                }
            }
        }
    }

    fn whitespace(&mut self, start: usize) {
        self.eat_while(|c| c == ' ');
        self.push(TokenKind::Whitespace, start);
    }

    fn comment(&mut self, start: usize) {
        self.eat_while(|c| !is_line_terminator(c));
        self.push(TokenKind::Comment, start);
    }

    fn line_terminator(&mut self, c: char, start: usize) {
        self.bump(c);
        if c == '\r' && self.peek() == Some('\n') {
            self.bump('\n');
        }
        let kind = if self.line_has_tokens {
            TokenKind::Newline
        } else {
            TokenKind::BlankLine
        };
        self.push(kind, start);
        self.line_has_tokens = false;
    }

    fn at_sign(&mut self, start: usize) {
        self.single(TokenKind::AtSign, '@', start);
    }

    fn colon(&mut self, start: usize) {
        self.single(TokenKind::Colon, ':', start);
    }

    fn comma(&mut self, start: usize) {
        self.single(TokenKind::Comma, ',', start);
    }

    fn string(&mut self, start: usize) {
        self.bump('"');
        let mut invalid_escapes = Vec::new();
        let terminated = loop {
            match self.peek() {
                Some('"') => {
                    self.bump('"');
                    break true;
                }
                Some('\\') => {
                    let escape_start = self.pos;
                    self.bump('\\');
                    // The escaped character belongs to the string, unless the line ends instead.
                    if let Some(c) = self.peek()
                        && !is_line_terminator(c)
                    {
                        self.bump(c);
                        if !matches!(c, '"' | '\\') {
                            invalid_escapes.push((escape_start, self.pos));
                        }
                    }
                }
                Some(c) if !is_line_terminator(c) => self.bump(c),
                _ => break false,
            }
        };
        if terminated {
            // An unterminated string is reported only as unterminated, so its escapes are reported
            // only once the closing quote is found.
            for (escape_start, escape_end) in invalid_escapes {
                self.report(LexicalErrorKind::InvalidEscape, escape_start, escape_end);
            }
            self.push(TokenKind::String, start);
        } else {
            self.push_error(LexicalErrorKind::UnterminatedString, start);
        }
    }

    fn word(&mut self, start: usize) {
        self.eat_while(is_word_char);
        self.push(TokenKind::Word, start);
    }

    fn single(&mut self, kind: TokenKind, c: char, start: usize) {
        self.bump(c);
        self.push(kind, start);
    }

    fn rest(&self) -> &str {
        &self.source[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self, c: char) {
        self.pos += c.len_utf8();
    }

    fn eat_while(&mut self, predicate: impl Fn(char) -> bool) {
        while let Some(c) = self.peek() {
            if !predicate(c) {
                break;
            }
            self.bump(c);
        }
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        if !kind.is_ignored() {
            self.line_has_tokens = true;
        }
        let len = to_u32(self.pos - start);
        self.tokens.push(Token { kind, len });
    }

    fn report(&mut self, kind: LexicalErrorKind, start: usize, end: usize) {
        let error = LexicalError {
            kind,
            offset: to_u32(start),
            len: to_u32(end - start),
        };
        self.errors.push(error);
    }

    fn push_error(&mut self, kind: LexicalErrorKind, start: usize) {
        self.report(kind, start, self.pos);
        self.push(TokenKind::Error, start);
    }
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r')
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use LexicalErrorKind::{InvalidEscape, UnexpectedCharacter, UnterminatedString};
    use TokenKind::{
        AtSign, BlankLine, ByteOrderMark, Colon, Comma, Comment, Error, Newline, Whitespace, Word,
    };

    fn check_lexer(text: &str, tokens: &[(TokenKind, &str)], errors: &[LexicalError]) {
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

    fn error(kind: LexicalErrorKind, offset: u32, len: u32) -> LexicalError {
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
    fn each_unexpected_character_is_its_own_error_token() {
        // "é" is one character but two bytes in UTF-8.
        check_lexer(
            "-é_",
            &[(Error, "-"), (Error, "é"), (Error, "_")],
            &[
                error(UnexpectedCharacter, 0, 1),
                error(UnexpectedCharacter, 1, 2),
                error(UnexpectedCharacter, 3, 1),
            ],
        );
    }

    #[test]
    fn errors_are_reported_in_source_order() {
        check_lexer(
            "- \"x",
            &[(Error, "-"), (Whitespace, " "), (Error, "\"x")],
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
    fn leading_byte_order_mark_is_its_own_token() {
        check_lexer(
            "\u{FEFF}a",
            &[(ByteOrderMark, "\u{FEFF}"), (Word, "a")],
            &[],
        );
    }

    #[test]
    fn byte_order_mark_elsewhere_is_an_error() {
        check_lexer(
            "a\u{FEFF}",
            &[(Word, "a"), (Error, "\u{FEFF}")],
            &[error(UnexpectedCharacter, 1, 3)],
        );
    }

    #[test]
    fn only_the_first_byte_order_mark_is_ignored() {
        check_lexer(
            "\u{FEFF}\u{FEFF}",
            &[(ByteOrderMark, "\u{FEFF}"), (Error, "\u{FEFF}")],
            &[error(UnexpectedCharacter, 3, 3)],
        );
    }

    #[test]
    fn byte_order_mark_survives_decoding() {
        assert_eq!(lex_bytes(b"\xef\xbb\xbfa"), lex("\u{FEFF}a"));
    }

    // Whitespace

    #[test]
    fn a_run_of_spaces_is_one_whitespace_token() {
        check_lexer("   ", &[(Whitespace, "   ")], &[]);
    }

    #[test]
    fn whitespace_sits_between_other_tokens() {
        check_lexer("a  b", &[(Word, "a"), (Whitespace, "  "), (Word, "b")], &[]);
    }

    #[test]
    fn a_tab_is_not_whitespace() {
        check_lexer(
            " \t ",
            &[(Whitespace, " "), (Error, "\t"), (Whitespace, " ")],
            &[error(UnexpectedCharacter, 1, 1)],
        );
    }

    #[test]
    fn a_comment_runs_to_the_end_of_the_line() {
        check_lexer("# a note", &[(Comment, "# a note")], &[]);
    }

    #[test]
    fn a_comment_stops_before_a_line_terminator() {
        for terminator in ["\n", "\r", "\r\n"] {
            check_lexer(
                &format!("# a{terminator}b"),
                &[(Comment, "# a"), (BlankLine, terminator), (Word, "b")],
                &[],
            );
        }
    }

    #[test]
    fn a_lone_hash_is_a_comment() {
        check_lexer("#", &[(Comment, "#")], &[]);
    }

    #[test]
    fn a_comment_may_follow_other_tokens() {
        check_lexer(
            "a # x",
            &[(Word, "a"), (Whitespace, " "), (Comment, "# x")],
            &[],
        );
    }

    #[test]
    fn a_comment_may_contain_anything_but_a_line_terminator() {
        check_lexer("#\t@\"é#", &[(Comment, "#\t@\"é#")], &[]);
    }

    #[test]
    fn a_line_feed_after_tokens_is_a_newline() {
        check_lexer("a\n", &[(Word, "a"), (Newline, "\n")], &[]);
    }

    #[test]
    fn a_line_feed_on_an_empty_line_is_a_blank_line() {
        check_lexer("\n", &[(BlankLine, "\n")], &[]);
    }

    #[test]
    fn a_line_of_whitespace_and_a_comment_is_a_blank_line() {
        check_lexer(
            "  # x\n",
            &[(Whitespace, "  "), (Comment, "# x"), (BlankLine, "\n")],
            &[],
        );
    }

    #[test]
    fn a_line_with_only_a_byte_order_mark_is_a_blank_line() {
        check_lexer(
            "\u{FEFF}\n",
            &[(ByteOrderMark, "\u{FEFF}"), (BlankLine, "\n")],
            &[],
        );
    }

    #[test]
    fn a_comment_after_tokens_keeps_the_newline() {
        check_lexer(
            "a # x\n",
            &[
                (Word, "a"),
                (Whitespace, " "),
                (Comment, "# x"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_line_with_an_error_ends_in_a_newline() {
        check_lexer(
            "-\n",
            &[(Error, "-"), (Newline, "\n")],
            &[error(UnexpectedCharacter, 0, 1)],
        );
    }

    #[test]
    fn each_line_is_judged_on_its_own_tokens() {
        check_lexer(
            "a\n\nb\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (BlankLine, "\n"),
                (Word, "b"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn every_line_terminator_ends_a_line() {
        for terminator in ["\n", "\r\n", "\r"] {
            check_lexer(
                &format!("a{terminator}{terminator}"),
                &[(Word, "a"), (Newline, terminator), (BlankLine, terminator)],
                &[],
            );
        }
    }

    #[test]
    fn a_carriage_return_and_line_feed_are_one_terminator() {
        check_lexer("\r\n", &[(BlankLine, "\r\n")], &[]);
    }

    #[test]
    fn a_line_feed_before_a_carriage_return_is_two_terminators() {
        check_lexer("\n\r", &[(BlankLine, "\n"), (BlankLine, "\r")], &[]);
    }

    #[test]
    fn a_carriage_return_before_a_crlf_is_two_terminators() {
        check_lexer(
            "a\r\r\n",
            &[(Word, "a"), (Newline, "\r"), (BlankLine, "\r\n")],
            &[],
        );
    }

    #[test]
    fn terminators_may_be_mixed_in_one_document() {
        check_lexer(
            "a\r\nb\rc\n",
            &[
                (Word, "a"),
                (Newline, "\r\n"),
                (Word, "b"),
                (Newline, "\r"),
                (Word, "c"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_comment_line_ending_in_crlf_is_a_blank_line() {
        check_lexer("# x\r\n", &[(Comment, "# x"), (BlankLine, "\r\n")], &[]);
    }

    #[test]
    fn each_punctuator_is_its_own_token() {
        check_lexer("@", &[(AtSign, "@")], &[]);
        check_lexer(":", &[(Colon, ":")], &[]);
        check_lexer(",", &[(Comma, ",")], &[]);
    }

    #[test]
    fn adjacent_punctuators_are_separate_tokens() {
        check_lexer("@:,", &[(AtSign, "@"), (Colon, ":"), (Comma, ",")], &[]);
    }

    #[test]
    fn a_line_with_a_punctuator_ends_in_a_newline() {
        for (punctuator, kind) in [("@", AtSign), (":", Colon), (",", Comma)] {
            check_lexer(
                &format!("{punctuator}\n"),
                &[(kind, punctuator), (Newline, "\n")],
                &[],
            );
        }
    }

    #[test]
    fn punctuators_inside_a_comment_are_part_of_it() {
        check_lexer("# @:,", &[(Comment, "# @:,")], &[]);
    }

    #[test]
    fn a_keyword_is_a_word() {
        check_lexer("contest", &[(Word, "contest")], &[]);
    }

    #[test]
    fn a_hyphenated_keyword_is_one_word() {
        check_lexer("no-position", &[(Word, "no-position")], &[]);
    }

    #[test]
    fn a_word_continues_through_letters_digits_hyphens_and_underscores() {
        for text in ["if_yes", "x1", "A-b_C", "z-9_"] {
            check_lexer(text, &[(Word, text)], &[]);
        }
    }

    #[test]
    fn an_identifier_is_an_at_sign_and_a_word() {
        check_lexer(
            "@dana-lombardi",
            &[(AtSign, "@"), (Word, "dana-lombardi")],
            &[],
        );
    }

    #[test]
    fn a_word_cannot_start_with_a_hyphen() {
        check_lexer(
            "@-lombardi",
            &[(AtSign, "@"), (Error, "-"), (Word, "lombardi")],
            &[error(UnexpectedCharacter, 1, 1)],
        );
    }

    #[test]
    fn a_word_cannot_start_with_an_underscore() {
        check_lexer(
            "_x",
            &[(Error, "_"), (Word, "x")],
            &[error(UnexpectedCharacter, 0, 1)],
        );
    }

    #[test]
    fn letters_in_a_word_are_ascii_only() {
        check_lexer(
            "café",
            &[(Word, "caf"), (Error, "é")],
            &[error(UnexpectedCharacter, 3, 2)],
        );
    }

    #[test]
    fn a_word_cannot_start_with_a_digit() {
        check_lexer(
            "9abc",
            &[(Error, "9"), (Word, "abc")],
            &[error(UnexpectedCharacter, 0, 1)],
        );
    }

    #[test]
    fn words_are_separated_by_whitespace_and_punctuators() {
        check_lexer(
            "a b,c:d",
            &[
                (Word, "a"),
                (Whitespace, " "),
                (Word, "b"),
                (Comma, ","),
                (Word, "c"),
                (Colon, ":"),
                (Word, "d"),
            ],
            &[],
        );
    }

    #[test]
    fn a_line_with_a_word_ends_in_a_newline() {
        check_lexer("guide\n", &[(Word, "guide"), (Newline, "\n")], &[]);
    }

    #[test]
    fn a_whole_string_is_one_token() {
        for text in [
            r#""Vote for one""#,
            r#""""#,
            r#""a \"b\" c""#,
            r#""a \\ b""#,
            r##""# @ : , é""##,
        ] {
            check_lexer(text, &[(TokenKind::String, text)], &[]);
        }
    }

    #[test]
    fn an_escaped_backslash_before_the_closing_quote_does_not_escape_it() {
        check_lexer(
            r#""a\\" x"#,
            &[
                (TokenKind::String, r#""a\\""#),
                (Whitespace, " "),
                (Word, "x"),
            ],
            &[],
        );
    }

    #[test]
    fn an_invalid_escape_is_still_a_string() {
        // The specification's example for the invalid-escape error: the error spans the backslash
        // and the character after it, and the token is still a String.
        check_lexer(
            r#""Measure \#3""#,
            &[(TokenKind::String, r#""Measure \#3""#)],
            &[error(InvalidEscape, 9, 2)],
        );
    }

    #[test]
    fn any_character_but_a_quote_or_backslash_makes_an_invalid_escape() {
        // A backslash and "n" is not a line break in Oval, just an invalid escape.
        check_lexer(
            r#""a\nb""#,
            &[(TokenKind::String, r#""a\nb""#)],
            &[error(InvalidEscape, 2, 2)],
        );
    }

    #[test]
    fn an_invalid_escape_spans_a_whole_multibyte_character() {
        // "é" is two bytes, so the escape is three.
        check_lexer(
            r#""\é""#,
            &[(TokenKind::String, r#""\é""#)],
            &[error(InvalidEscape, 1, 3)],
        );
    }

    #[test]
    fn each_invalid_escape_is_reported() {
        check_lexer(
            r#""\a\b""#,
            &[(TokenKind::String, r#""\a\b""#)],
            &[error(InvalidEscape, 1, 2), error(InvalidEscape, 3, 2)],
        );
    }

    #[test]
    fn an_invalid_escape_is_reported_where_it_is() {
        check_lexer(
            r#"x "\#""#,
            &[
                (Word, "x"),
                (Whitespace, " "),
                (TokenKind::String, r#""\#""#),
            ],
            &[error(InvalidEscape, 3, 2)],
        );
    }

    #[test]
    fn an_unterminated_string_reports_no_invalid_escapes() {
        check_lexer(
            "\"\\#\n",
            &[(Error, "\"\\#"), (Newline, "\n")],
            &[error(UnterminatedString, 0, 3)],
        );
    }

    #[test]
    fn an_unterminated_string_is_an_error_up_to_the_line_terminator() {
        check_lexer(
            "\"November 3\ndate",
            &[(Error, "\"November 3"), (Newline, "\n"), (Word, "date")],
            &[error(UnterminatedString, 0, 11)],
        );
    }

    #[test]
    fn an_unterminated_string_stops_before_a_carriage_return() {
        check_lexer(
            "\"a\r\n",
            &[(Error, "\"a"), (Newline, "\r\n")],
            &[error(UnterminatedString, 0, 2)],
        );
    }

    #[test]
    fn an_unterminated_string_may_end_at_the_end_of_input() {
        check_lexer(
            "\"abc",
            &[(Error, "\"abc")],
            &[error(UnterminatedString, 0, 4)],
        );
    }

    #[test]
    fn an_unterminated_string_is_reported_where_it_starts() {
        check_lexer(
            "a \"b",
            &[(Word, "a"), (Whitespace, " "), (Error, "\"b")],
            &[error(UnterminatedString, 2, 2)],
        );
    }

    #[test]
    fn a_backslash_before_a_line_terminator_is_part_of_an_unterminated_string() {
        check_lexer(
            "\"x\\\n",
            &[(Error, "\"x\\"), (Newline, "\n")],
            &[error(UnterminatedString, 0, 3)],
        );
    }

    #[test]
    fn a_backslash_at_the_end_of_input_is_part_of_an_unterminated_string() {
        check_lexer(
            "\"x\\",
            &[(Error, "\"x\\")],
            &[error(UnterminatedString, 0, 3)],
        );
    }

    #[test]
    fn a_field_with_a_string_value() {
        check_lexer(
            r#"subtitle "Vote for one""#,
            &[
                (Word, "subtitle"),
                (Whitespace, " "),
                (TokenKind::String, r#""Vote for one""#),
            ],
            &[],
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
