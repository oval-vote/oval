use crate::parser::Parser;
use crate::{Expected, SyntaxErrorKind, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn identifier(&mut self) {
        if self.at(TokenKind::AtSign) {
            self.start_node(SyntaxKind::Identifier);
            self.bump();
            if !self.eat(TokenKind::Word) && !self.error_token_ahead() {
                self.report(
                    SyntaxErrorKind::MissingIdentifier,
                    self.offset,
                    self.offset,
                    vec![Expected::Token(TokenKind::Word)],
                );
            }
            self.finish_node();
        } else if self.at_line_end() || self.at(TokenKind::String) {
            if !self.error_token_ahead() {
                let offset = match self.peek() {
                    Some((token, offset)) if token.kind == TokenKind::String => offset,
                    _ => self.line_end,
                };
                self.report(
                    SyntaxErrorKind::MissingIdentifier,
                    offset,
                    offset,
                    vec![Expected::Token(TokenKind::AtSign)],
                );
            }
        } else {
            self.skip_line(vec![Expected::Token(TokenKind::AtSign)]);
        }
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

    fn missing(offset: u32, expected: TokenKind) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingIdentifier,
            offset,
            len: 0,
            expected: vec![Expected::Token(expected)],
        }
    }

    #[test]
    fn an_identifier_is_an_at_sign_and_a_word() {
        check_errors("party @dem \"Democratic\"\n", &[]);
        check(
            "party @dem \"Democratic\"\n",
            &expect![[r#"
            Document@0..24
              Party@0..24
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..10
                  AtSign@6..7 "@"
                  Word@7..10 "dem"
                Whitespace@10..11 " "
                String@11..23 "\"Democratic\""
                Newline@23..24 "\n"
        "#]],
        );
    }

    #[test]
    fn whitespace_may_separate_the_at_sign_from_its_word() {
        check_errors("party @ dem \"Democratic\"\n", &[]);
        check(
            "party @ dem \"Democratic\"\n",
            &expect![[r#"
            Document@0..25
              Party@0..25
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..11
                  AtSign@6..7 "@"
                  Whitespace@7..8 " "
                  Word@8..11 "dem"
                Whitespace@11..12 " "
                String@12..24 "\"Democratic\""
                Newline@24..25 "\n"
        "#]],
        );
    }

    #[test]
    fn a_keyword_after_an_at_sign_is_an_ordinary_identifier() {
        check_errors("party @ballot \"Ballot Party\"\n", &[]);
    }

    #[test]
    fn a_string_where_the_identifier_belongs_is_missing_an_identifier() {
        let text = "party \"Democratic\"\n";
        check_errors(text, &[missing(6, TokenKind::AtSign)]);
        check(
            text,
            &expect![[r#"
            Document@0..19
              Party@0..19
                Word@0..5 "party"
                Whitespace@5..6 " "
                String@6..18 "\"Democratic\""
                Newline@18..19 "\n"
        "#]],
        );
    }

    #[test]
    fn an_at_sign_without_a_word_is_missing_an_identifier_after_it() {
        let text = "party @ \"Democratic\"\n";
        check_errors(text, &[missing(7, TokenKind::Word)]);
        check(
            text,
            &expect![[r#"
            Document@0..21
              Party@0..21
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..7
                  AtSign@6..7 "@"
                Whitespace@7..8 " "
                String@8..20 "\"Democratic\""
                Newline@20..21 "\n"
        "#]],
        );
    }

    #[test]
    fn an_at_sign_at_the_end_of_the_line_is_missing_an_identifier_after_it() {
        check_errors(
            "party @\n",
            &[
                missing(7, TokenKind::Word),
                SyntaxError {
                    kind: SyntaxErrorKind::MissingName,
                    offset: 7,
                    len: 0,
                    expected: vec![Expected::Token(TokenKind::String)],
                },
            ],
        );
    }

    #[test]
    fn another_token_where_the_identifier_belongs_is_an_unexpected_token() {
        check_errors(
            "party 5 \"Democratic\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedToken,
                offset: 6,
                len: 14,
                expected: vec![Expected::Token(TokenKind::AtSign)],
            }],
        );
    }

    #[test]
    fn a_missing_identifier_is_not_reported_where_an_error_token_is() {
        for text in ["party ~ \"Democratic\"\n", "party @~\n"] {
            let result = parse(text).unwrap();
            assert_eq!(result.syntax_errors, [], "{text:?}");
            assert_eq!(result.lexical_errors.len(), 1, "{text:?}");
        }
    }
}
