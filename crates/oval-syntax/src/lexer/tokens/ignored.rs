use crate::lexer::{BYTE_ORDER_MARK, Lexer, is_line_terminator};
use crate::{LexicalErrorKind, TokenKind};

impl Lexer<'_> {
    pub(in crate::lexer) fn byte_order_mark(&mut self, start: usize) {
        self.bump(BYTE_ORDER_MARK);
        self.push(TokenKind::ByteOrderMark, start);
        self.line_start = self.pos;
    }

    pub(in crate::lexer) fn whitespace(&mut self, start: usize) {
        self.eat_while(|c| c == ' ');
        self.push(TokenKind::Whitespace, start);
    }

    pub(in crate::lexer) fn illegal_tab(&mut self, start: usize) {
        self.eat_while(|c| c == '\t');
        self.push_error(LexicalErrorKind::IllegalTab, start);
    }

    pub(in crate::lexer) fn comment(&mut self, start: usize) {
        self.eat_while(|c| !is_line_terminator(c));
        self.push(TokenKind::Comment, start);
    }
}

#[cfg(test)]
mod tests {
    use crate::LexicalErrorKind::{IllegalTab, UnexpectedCharacter};
    use crate::TokenKind::{BlankLine, ByteOrderMark, Comment, Error, Newline, Whitespace, Word};
    use crate::lexer::tests::{check_lexer, error};
    use crate::{TokenKind, lex, lex_bytes};

    #[test]
    fn leading_byte_order_mark_is_its_own_token() {
        check_lexer(
            "\u{FEFF}a",
            &[(ByteOrderMark, "\u{FEFF}"), (Word, "a"), (Newline, "")],
            &[],
        );
    }

    #[test]
    fn byte_order_mark_elsewhere_is_an_error() {
        check_lexer(
            "a\u{FEFF}",
            &[(Word, "a"), (Error, "\u{FEFF}"), (Newline, "")],
            &[error(UnexpectedCharacter, 1, 3)],
        );
    }

    #[test]
    fn only_the_first_byte_order_mark_is_ignored() {
        check_lexer(
            "\u{FEFF}\u{FEFF}",
            &[
                (ByteOrderMark, "\u{FEFF}"),
                (Error, "\u{FEFF}"),
                (Newline, ""),
            ],
            &[error(UnexpectedCharacter, 3, 3)],
        );
    }

    #[test]
    fn byte_order_mark_survives_decoding() {
        assert_eq!(lex_bytes(b"\xef\xbb\xbfa"), lex("\u{FEFF}a"));
    }

    #[test]
    fn a_run_of_spaces_is_one_whitespace_token() {
        check_lexer("   ", &[(Whitespace, "   ")], &[]);
    }

    #[test]
    fn whitespace_sits_between_other_tokens() {
        check_lexer(
            "a  b",
            &[(Word, "a"), (Whitespace, "  "), (Word, "b"), (Newline, "")],
            &[],
        );
    }

    #[test]
    fn a_tab_is_not_whitespace() {
        check_lexer(
            "a \t b",
            &[
                (Word, "a"),
                (Whitespace, " "),
                (Error, "\t"),
                (Whitespace, " "),
                (Word, "b"),
                (Newline, ""),
            ],
            &[error(IllegalTab, 2, 1)],
        );
    }

    #[test]
    fn a_run_of_tabs_is_one_error() {
        check_lexer(
            "\t\t\ta",
            &[(Error, "\t\t\t"), (Word, "a"), (Newline, "")],
            &[error(IllegalTab, 0, 3)],
        );
    }

    #[test]
    fn tabs_separated_by_a_space_are_two_errors() {
        check_lexer(
            "\t \t",
            &[
                (Error, "\t"),
                (Whitespace, " "),
                (Error, "\t"),
                (Newline, ""),
            ],
            &[error(IllegalTab, 0, 1), error(IllegalTab, 2, 1)],
        );
    }

    #[test]
    fn a_tab_between_tokens_is_an_error() {
        // The specification's example for the illegal-tab error.
        check_lexer(
            "subtitle\t\"Vote for one\"",
            &[
                (Word, "subtitle"),
                (Error, "\t"),
                (TokenKind::String, "\"Vote for one\""),
                (Newline, ""),
            ],
            &[error(IllegalTab, 8, 1)],
        );
    }

    #[test]
    fn a_tab_inside_a_string_is_part_of_it() {
        check_lexer(
            "\"a\tb\"",
            &[(TokenKind::String, "\"a\tb\""), (Newline, "")],
            &[],
        );
    }

    #[test]
    fn a_tab_inside_a_comment_is_part_of_it() {
        check_lexer("# a\tb", &[(Comment, "# a\tb")], &[]);
    }

    #[test]
    fn a_line_with_only_a_tab_ends_in_a_newline() {
        // An error is a token, so the line is not blank.
        check_lexer(
            "\t\n",
            &[(Error, "\t"), (Newline, "\n")],
            &[error(IllegalTab, 0, 1)],
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
                &[
                    (Comment, "# a"),
                    (BlankLine, terminator),
                    (Word, "b"),
                    (Newline, ""),
                ],
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
            &[
                (Word, "a"),
                (Whitespace, " "),
                (Comment, "# x"),
                (Newline, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_comment_may_contain_anything_but_a_line_terminator() {
        check_lexer("#\t@\"é#", &[(Comment, "#\t@\"é#")], &[]);
    }
}
