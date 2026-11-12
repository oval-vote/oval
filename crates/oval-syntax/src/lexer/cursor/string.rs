use super::{Cursor, is_line_terminator};
use crate::{LexicalErrorKind, TokenKind};

impl Cursor<'_> {
    pub(super) fn string(&mut self, start: usize) {
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
            for (escape_start, escape_end) in invalid_escapes {
                self.report(LexicalErrorKind::InvalidEscape, escape_start, escape_end);
            }
            self.push(TokenKind::String, start);
        } else {
            self.push_error(LexicalErrorKind::UnterminatedString, start);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::LexicalErrorKind::{InvalidEscape, UnterminatedString};
    use crate::TokenKind;
    use crate::TokenKind::{Error, Newline, Whitespace, Word};
    use crate::lexer::tests::{check_lexer, error};

    #[test]
    fn a_whole_string_is_one_token() {
        for text in [
            r#""Vote for one""#,
            r#""""#,
            r#""a \"b\" c""#,
            r#""a \\ b""#,
            r##""# @ : , é""##,
        ] {
            check_lexer(text, &[(TokenKind::String, text), (Newline, "")], &[]);
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
                (Newline, ""),
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
            &[(TokenKind::String, r#""Measure \#3""#), (Newline, "")],
            &[error(InvalidEscape, 9, 2)],
        );
    }

    #[test]
    fn any_character_but_a_quote_or_backslash_makes_an_invalid_escape() {
        // A backslash and "n" is not a line break in Oval, just an invalid escape.
        check_lexer(
            r#""a\nb""#,
            &[(TokenKind::String, r#""a\nb""#), (Newline, "")],
            &[error(InvalidEscape, 2, 2)],
        );
    }

    #[test]
    fn an_invalid_escape_spans_a_whole_multibyte_character() {
        // "é" is two bytes, so the escape is three.
        check_lexer(
            r#""\é""#,
            &[(TokenKind::String, r#""\é""#), (Newline, "")],
            &[error(InvalidEscape, 1, 3)],
        );
    }

    #[test]
    fn each_invalid_escape_is_reported() {
        check_lexer(
            r#""\a\b""#,
            &[(TokenKind::String, r#""\a\b""#), (Newline, "")],
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
                (Newline, ""),
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
            &[
                (Error, "\"November 3"),
                (Newline, "\n"),
                (Word, "date"),
                (Newline, ""),
            ],
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
            &[(Error, "\"abc"), (Newline, "")],
            &[error(UnterminatedString, 0, 4)],
        );
    }

    #[test]
    fn an_unterminated_string_is_reported_where_it_starts() {
        check_lexer(
            "a \"b",
            &[
                (Word, "a"),
                (Whitespace, " "),
                (Error, "\"b"),
                (Newline, ""),
            ],
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
            &[(Error, "\"x\\"), (Newline, "")],
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
                (Newline, ""),
            ],
            &[],
        );
    }
}
