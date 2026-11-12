use crate::lexer::{Lexer, is_date_shaped, is_word_char};
use crate::{LexicalErrorKind, TokenKind};

impl Lexer<'_> {
    pub(in crate::lexer) fn number(&mut self, start: usize) {
        self.eat_while(|c| is_word_char(c) || c == '.');
        let text = &self.source[start..self.pos];
        if text.bytes().all(|b| b.is_ascii_digit()) {
            self.push(TokenKind::Integer, start);
        } else if is_date_shaped(text) {
            self.push(TokenKind::Date, start);
        } else {
            self.push_error(LexicalErrorKind::MalformedNumber, start);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::LexicalErrorKind::{MalformedNumber, UnexpectedCharacter};
    use crate::TokenKind::{AtSign, Colon, Comma, Date, Error, Integer, Newline, Whitespace, Word};
    use crate::lexer::tests::{check_lexer, error};

    #[test]
    fn digits_are_an_integer() {
        for text in ["0", "1", "42", "2026"] {
            check_lexer(text, &[(Integer, text), (Newline, "")], &[]);
        }
    }

    #[test]
    fn leading_zeros_are_part_of_an_integer() {
        check_lexer("007", &[(Integer, "007"), (Newline, "")], &[]);
    }

    #[test]
    fn a_date_has_the_shape_yyyy_mm_dd() {
        check_lexer("2026-11-03", &[(Date, "2026-11-03"), (Newline, "")], &[]);
    }

    #[test]
    fn a_date_is_checked_for_its_shape_only() {
        // February 30 does not exist, but that is a validation error, not a lexical one.
        check_lexer("2026-02-30", &[(Date, "2026-02-30"), (Newline, "")], &[]);
    }

    #[test]
    fn a_run_that_is_neither_an_integer_nor_a_date_is_one_malformed_number() {
        // The specification's examples of malformed numbers.
        for text in ["12abc", "3_000", "2026-11-5", "1.1", "1.x"] {
            let len = u32::try_from(text.len()).unwrap();
            check_lexer(
                text,
                &[(Error, text), (Newline, "")],
                &[error(MalformedNumber, 0, len)],
            );
        }
    }

    #[test]
    fn a_date_with_the_wrong_shape_is_a_malformed_number() {
        for text in [
            "2026-11-3",
            "26-11-03",
            "2026-1103",
            "2026-11-033",
            "2026-11-03x",
        ] {
            let len = u32::try_from(text.len()).unwrap();
            check_lexer(
                text,
                &[(Error, text), (Newline, "")],
                &[error(MalformedNumber, 0, len)],
            );
        }
    }

    #[test]
    fn a_number_stops_at_whitespace_and_punctuators() {
        check_lexer(
            "1 2,3:4@5",
            &[
                (Integer, "1"),
                (Whitespace, " "),
                (Integer, "2"),
                (Comma, ","),
                (Integer, "3"),
                (Colon, ":"),
                (Integer, "4"),
                (AtSign, "@"),
                (Integer, "5"),
                (Newline, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_number_stops_before_a_character_outside_ascii() {
        check_lexer(
            "1é",
            &[(Integer, "1"), (Error, "é"), (Newline, "")],
            &[error(UnexpectedCharacter, 1, 2)],
        );
    }

    #[test]
    fn the_version_marker_is_a_word_and_an_integer() {
        check_lexer(
            "oval 1\n",
            &[
                (Word, "oval"),
                (Whitespace, " "),
                (Integer, "1"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_date_field() {
        // The specification's example for the malformed-number error.
        check_lexer(
            "date 2026-11-03\n",
            &[
                (Word, "date"),
                (Whitespace, " "),
                (Date, "2026-11-03"),
                (Newline, "\n"),
            ],
            &[],
        );
    }
}
