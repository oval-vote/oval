use super::{Cursor, is_word_char};
use crate::TokenKind;

impl Cursor<'_> {
    pub(super) fn word(&mut self, start: usize) {
        self.eat_while(is_word_char);
        self.push(TokenKind::Word, start);
    }
}

#[cfg(test)]
mod tests {
    use crate::LexicalErrorKind::{MalformedNumber, UnexpectedCharacter};
    use crate::TokenKind::{AtSign, Colon, Comma, Error, Newline, Whitespace, Word};
    use crate::lexer::tests::{check_lexer, error};

    #[test]
    fn a_keyword_is_a_word() {
        check_lexer("contest", &[(Word, "contest"), (Newline, "")], &[]);
    }

    #[test]
    fn a_hyphenated_keyword_is_one_word() {
        check_lexer("no-position", &[(Word, "no-position"), (Newline, "")], &[]);
    }

    #[test]
    fn a_word_continues_through_letters_digits_hyphens_and_underscores() {
        for text in ["if_yes", "x1", "A-b_C", "z-9_"] {
            check_lexer(text, &[(Word, text), (Newline, "")], &[]);
        }
    }

    #[test]
    fn an_identifier_is_an_at_sign_and_a_word() {
        check_lexer(
            "@dana-lombardi",
            &[(AtSign, "@"), (Word, "dana-lombardi"), (Newline, "")],
            &[],
        );
    }

    #[test]
    fn a_word_cannot_start_with_a_hyphen() {
        check_lexer(
            "@-lombardi",
            &[
                (AtSign, "@"),
                (Error, "-"),
                (Word, "lombardi"),
                (Newline, ""),
            ],
            &[error(UnexpectedCharacter, 1, 1)],
        );
    }

    #[test]
    fn a_word_cannot_start_with_an_underscore() {
        check_lexer(
            "_x",
            &[(Error, "_"), (Word, "x"), (Newline, "")],
            &[error(UnexpectedCharacter, 0, 1)],
        );
    }

    #[test]
    fn letters_in_a_word_are_ascii_only() {
        check_lexer(
            "café",
            &[(Word, "caf"), (Error, "é"), (Newline, "")],
            &[error(UnexpectedCharacter, 3, 2)],
        );
    }

    #[test]
    fn a_word_cannot_start_with_a_digit() {
        // A run that starts with a digit is a number, so this is one malformed number.
        check_lexer(
            "9abc",
            &[(Error, "9abc"), (Newline, "")],
            &[error(MalformedNumber, 0, 4)],
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
                (Newline, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_line_with_a_word_ends_in_a_newline() {
        check_lexer("guide\n", &[(Word, "guide"), (Newline, "\n")], &[]);
    }
}
