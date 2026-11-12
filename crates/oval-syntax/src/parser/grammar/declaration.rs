use crate::parser::Parser;
use crate::{Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn declaration(&mut self) -> bool {
        match self.current_keyword() {
            Some(Keyword::Party) => self.party(),
            _ => return false,
        }
        true
    }

    fn party(&mut self) {
        self.start_node(SyntaxKind::Party);
        self.bump();
        self.identifier();
        self.expect(TokenKind::String, SyntaxErrorKind::MissingName);
        self.end_of_line();
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
    fn declarations_follow_the_guide_fields() {
        let text = "guide \"G\"\n\nparty @dem \"Democratic\"\nparty @rep \"Republican\"\n";
        check_errors(text, &[]);
        check(
            text,
            &expect![[r#"
            Document@0..59
              StringField@0..10
                Word@0..5 "guide"
                Whitespace@5..6 " "
                String@6..9 "\"G\""
                Newline@9..10 "\n"
              BlankLine@10..11 "\n"
              Party@11..35
                Word@11..16 "party"
                Whitespace@16..17 " "
                Identifier@17..21
                  AtSign@17..18 "@"
                  Word@18..21 "dem"
                Whitespace@21..22 " "
                String@22..34 "\"Democratic\""
                Newline@34..35 "\n"
              Party@35..59
                Word@35..40 "party"
                Whitespace@40..41 " "
                Identifier@41..45
                  AtSign@41..42 "@"
                  Word@42..45 "rep"
                Whitespace@45..46 " "
                String@46..58 "\"Republican\""
                Newline@58..59 "\n"
        "#]],
        );
    }

    #[test]
    fn a_party_without_a_name_is_missing_a_name() {
        let text = "party @dem\n";
        check_errors(
            text,
            &[error(
                SyntaxErrorKind::MissingName,
                10,
                0,
                TokenKind::String,
            )],
        );
        check(
            text,
            &expect![[r#"
            Document@0..11
              Party@0..11
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..10
                  AtSign@6..7 "@"
                  Word@7..10 "dem"
                Newline@10..11 "\n"
        "#]],
        );
    }

    #[test]
    fn a_bare_party_keyword_is_missing_its_identifier_and_its_name() {
        check_errors(
            "party\n",
            &[
                error(SyntaxErrorKind::MissingIdentifier, 5, 0, TokenKind::AtSign),
                error(SyntaxErrorKind::MissingName, 5, 0, TokenKind::String),
            ],
        );
    }

    #[test]
    fn a_name_of_the_wrong_kind_is_an_unexpected_token() {
        check_errors(
            "party @dem 5\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                11,
                1,
                TokenKind::String,
            )],
        );
    }

    #[test]
    fn a_token_after_the_name_is_an_unexpected_token() {
        check_errors(
            "party @dem \"Democratic\" extra\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                24,
                5,
                TokenKind::Newline,
            )],
        );
    }

    #[test]
    fn nothing_more_is_missing_on_a_skipped_line() {
        check_errors(
            "party 5\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                6,
                1,
                TokenKind::AtSign,
            )],
        );
    }
}
