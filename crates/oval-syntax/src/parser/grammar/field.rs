use crate::parser::Parser;
use crate::{Expected, SyntaxErrorKind, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn value_field(&mut self, node: SyntaxKind, value: TokenKind) {
        self.start_node(node);
        self.bump();
        self.expect(value, SyntaxErrorKind::MissingValue);
        self.end_of_line();
        self.finish_node();
    }

    pub(super) fn markdown_field(&mut self) {
        self.start_node(SyntaxKind::MarkdownField);
        self.bump();
        let has_colon = self.expect(TokenKind::Colon, SyntaxErrorKind::MissingValue);
        let line_has_error = self.line_has_error();
        if self.finish_line() {
            self.discard_body();
        } else if has_colon && !self.eat(TokenKind::MarkdownBlock) && !line_has_error {
            self.report(
                SyntaxErrorKind::MissingBody,
                self.offset,
                self.offset,
                vec![Expected::Token(TokenKind::MarkdownBlock)],
            );
        }
        self.finish_node();
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::tests::check;
    use crate::{Expected, SyntaxError, SyntaxErrorKind, TokenKind, parse};
    use expect_test::expect;

    fn check_errors(text: &str, expected: &[SyntaxError]) {
        assert_eq!(parse(text).unwrap().syntax_errors, expected, "{text:?}");
    }

    fn error(kind: SyntaxErrorKind, offset: u32, len: u32, expected: TokenKind) -> SyntaxError {
        SyntaxError {
            kind,
            offset,
            len,
            expected: vec![Expected::Token(expected)],
        }
    }

    #[test]
    fn a_markdown_field_holds_its_body() {
        check_errors("note:\n  Read *both* arguments.\n", &[]);
        check(
            "note:\n  Read *both* arguments.\n",
            &expect![[r#"
            Document@0..31
              MarkdownField@0..31
                Word@0..4 "note"
                Colon@4..5 ":"
                Newline@5..6 "\n"
                MarkdownBlock@6..31 "  Read *both* argumen ..."
        "#]],
        );
    }

    #[test]
    fn blank_lines_before_a_body_are_inside_the_field() {
        check(
            "note:\n\n  Text.\n",
            &expect![[r#"
            Document@0..15
              MarkdownField@0..15
                Word@0..4 "note"
                Colon@4..5 ":"
                Newline@5..6 "\n"
                BlankLine@6..7 "\n"
                MarkdownBlock@7..15 "  Text.\n"
        "#]],
        );
    }

    #[test]
    fn a_markdown_field_without_a_colon_is_missing_a_value() {
        check_errors(
            "note\n",
            &[error(SyntaxErrorKind::MissingValue, 4, 0, TokenKind::Colon)],
        );
    }

    #[test]
    fn a_value_where_the_colon_belongs_is_an_unexpected_token() {
        check_errors(
            "note \"x\"\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                5,
                3,
                TokenKind::Colon,
            )],
        );
    }

    #[test]
    fn text_after_the_colon_is_an_unexpected_token() {
        check_errors(
            "note: text\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                6,
                4,
                TokenKind::Newline,
            )],
        );
    }

    #[test]
    fn a_colon_without_a_body_is_missing_a_body() {
        let text = "note:\ndate 2026-11-03\n";
        check_errors(
            text,
            &[error(
                SyntaxErrorKind::MissingBody,
                6,
                0,
                TokenKind::MarkdownBlock,
            )],
        );
        check(
            text,
            &expect![[r#"
            Document@0..22
              MarkdownField@0..6
                Word@0..4 "note"
                Colon@4..5 ":"
                Newline@5..6 "\n"
              DateField@6..22
                Word@6..10 "date"
                Whitespace@10..11 " "
                Date@11..21 "2026-11-03"
                Newline@21..22 "\n"
        "#]],
        );
    }

    #[test]
    fn a_colon_at_the_end_of_input_is_missing_a_body() {
        check_errors(
            "note:",
            &[error(
                SyntaxErrorKind::MissingBody,
                5,
                0,
                TokenKind::MarkdownBlock,
            )],
        );
    }

    #[test]
    fn a_missing_body_is_not_reported_on_a_line_with_an_error_token() {
        let result = parse("note ~:\n").unwrap();
        assert_eq!(result.syntax_errors, []);
        assert_eq!(result.lexical_errors.len(), 1);
    }

    #[test]
    fn a_body_after_a_skipped_line_is_discarded_with_it() {
        let text = "guide \"Franklin Voter Guide\":\n  Written by the Franklin Civic Alliance.\n";
        check_errors(
            text,
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                28,
                1,
                TokenKind::Newline,
            )],
        );
        check(
            text,
            &expect![[r#"
            Document@0..72
              StringField@0..72
                Word@0..5 "guide"
                Whitespace@5..6 " "
                String@6..28 "\"Franklin Voter Guide\""
                ErrorNode@28..29
                  Colon@28..29 ":"
                Newline@29..30 "\n"
                ErrorNode@30..72
                  MarkdownBlock@30..72 "  Written by the Fran ..."
        "#]],
        );
    }

    #[test]
    fn a_body_after_a_markdown_field_with_a_skipped_line_is_discarded() {
        check_errors(
            "note \"x\":\n  Text.\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                5,
                4,
                TokenKind::Colon,
            )],
        );
        check_errors(
            "note::\n  Text.\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                5,
                1,
                TokenKind::Newline,
            )],
        );
        check_errors(
            "note: text:\n  Text.\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                6,
                5,
                TokenKind::Newline,
            )],
        );
        check(
            "note::\n  Text.\n",
            &expect![[r#"
            Document@0..15
              MarkdownField@0..15
                Word@0..4 "note"
                Colon@4..5 ":"
                ErrorNode@5..6
                  Colon@5..6 ":"
                Newline@6..7 "\n"
                ErrorNode@7..15
                  MarkdownBlock@7..15 "  Text.\n"
        "#]],
        );
    }
}
