use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const PARTY_FIELDS: [Expected; 3] = [
    Expected::Keyword(Keyword::Abbreviation),
    Expected::Keyword(Keyword::Website),
    Expected::Keyword(Keyword::Note),
];

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
        if self.at(TokenKind::Indent) {
            self.block(&PARTY_FIELDS, Self::party_field);
        }
        self.finish_node();
    }

    fn party_field(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };
        match keyword {
            Keyword::Abbreviation => self.field_once(seen, keyword, |p| {
                p.value_field(SyntaxKind::StringField, TokenKind::String);
            }),
            Keyword::Website => self.field_once(seen, keyword, |p| {
                p.value_field(SyntaxKind::UrlField, TokenKind::Url);
            }),
            Keyword::Note => self.field_once(seen, keyword, Self::markdown_field),
            _ => return false,
        }
        true
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
    fn a_party_block_holds_its_fields() {
        let text = "party @dem \"Democratic\"\n  abbreviation \"D\"\n  website https://dems.example\n  note:\n    Founded in 1828.\nparty @rep \"Republican\"\n";
        check_errors(text, &[]);
        check(
            text,
            &expect![[r#"
            Document@0..127
              Party@0..103
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..10
                  AtSign@6..7 "@"
                  Word@7..10 "dem"
                Whitespace@10..11 " "
                String@11..23 "\"Democratic\""
                Newline@23..24 "\n"
                Whitespace@24..26 "  "
                Block@26..103
                  Indent@26..26 ""
                  StringField@26..43
                    Word@26..38 "abbreviation"
                    Whitespace@38..39 " "
                    String@39..42 "\"D\""
                    Newline@42..43 "\n"
                  Whitespace@43..45 "  "
                  UrlField@45..74
                    Word@45..52 "website"
                    Whitespace@52..53 " "
                    Url@53..73 "https://dems.example"
                    Newline@73..74 "\n"
                  Whitespace@74..76 "  "
                  MarkdownField@76..103
                    Word@76..80 "note"
                    Colon@80..81 ":"
                    Newline@81..82 "\n"
                    MarkdownBlock@82..103 "    Founded in 1828.\n"
                  Dedent@103..103 ""
              Party@103..127
                Word@103..108 "party"
                Whitespace@108..109 " "
                Identifier@109..113
                  AtSign@109..110 "@"
                  Word@110..113 "rep"
                Whitespace@113..114 " "
                String@114..126 "\"Republican\""
                Newline@126..127 "\n"
        "#]],
        );
    }

    #[test]
    fn a_party_block_can_end_the_input() {
        check(
            "party @dem \"Democratic\"\n  abbreviation \"D\"",
            &expect![[r#"
                Document@0..42
                  Party@0..42
                    Word@0..5 "party"
                    Whitespace@5..6 " "
                    Identifier@6..10
                      AtSign@6..7 "@"
                      Word@7..10 "dem"
                    Whitespace@10..11 " "
                    String@11..23 "\"Democratic\""
                    Newline@23..24 "\n"
                    Whitespace@24..26 "  "
                    Block@26..42
                      Indent@26..26 ""
                      StringField@26..42
                        Word@26..38 "abbreviation"
                        Whitespace@38..39 " "
                        String@39..42 "\"D\""
                        Newline@42..42 ""
                      Dedent@42..42 ""
            "#]],
        );
    }

    #[test]
    fn ignored_lines_before_the_dedent_are_inside_the_block() {
        check(
            "party @dem \"Democratic\"\n  abbreviation \"D\"\n\n# aside\nparty @rep \"Republican\"\n",
            &expect![[r##"
                Document@0..76
                  Party@0..52
                    Word@0..5 "party"
                    Whitespace@5..6 " "
                    Identifier@6..10
                      AtSign@6..7 "@"
                      Word@7..10 "dem"
                    Whitespace@10..11 " "
                    String@11..23 "\"Democratic\""
                    Newline@23..24 "\n"
                    Whitespace@24..26 "  "
                    Block@26..52
                      Indent@26..26 ""
                      StringField@26..43
                        Word@26..38 "abbreviation"
                        Whitespace@38..39 " "
                        String@39..42 "\"D\""
                        Newline@42..43 "\n"
                      BlankLine@43..44 "\n"
                      Comment@44..51 "# aside"
                      BlankLine@51..52 "\n"
                      Dedent@52..52 ""
                  Party@52..76
                    Word@52..57 "party"
                    Whitespace@57..58 " "
                    Identifier@58..62
                      AtSign@58..59 "@"
                      Word@59..62 "rep"
                    Whitespace@62..63 " "
                    String@63..75 "\"Republican\""
                    Newline@75..76 "\n"
            "##]],
        );
    }

    #[test]
    fn a_block_of_only_error_tokens_is_empty() {
        let text = "party @dem \"Democratic\"\n  \t\n";
        let result = parse(text).unwrap();
        assert_eq!(result.lexical_errors.len(), 1);
        assert_eq!(result.syntax_errors, []);
        check(
            text,
            &expect![[r#"
            Document@0..28
              Party@0..28
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..10
                  AtSign@6..7 "@"
                  Word@7..10 "dem"
                Whitespace@10..11 " "
                String@11..23 "\"Democratic\""
                Newline@23..24 "\n"
                Whitespace@24..26 "  "
                Block@26..28
                  Indent@26..26 ""
                  Error@26..27 "\t"
                  Newline@27..28 "\n"
                  Dedent@28..28 ""
        "#]],
        );
    }

    #[test]
    fn a_repeated_field_in_a_party_block_is_set_aside() {
        check_errors(
            "party @dem \"Democratic\"\n  abbreviation \"D\"\n  abbreviation \"Dem\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::RepeatedField,
                offset: 45,
                len: 18,
                expected: Vec::new(),
            }],
        );
    }

    #[test]
    fn each_block_tracks_its_own_fields() {
        check_errors(
            "note:\n  Guide.\nparty @dem \"Democratic\"\n  note:\n    Dem.\nparty @rep \"Republican\"\n  note:\n    Rep.\n",
            &[],
        );
    }

    #[test]
    fn a_website_takes_a_url() {
        check_errors(
            "party @dem \"Democratic\"\n  website \"dems.example\"\n",
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                34,
                14,
                TokenKind::Url,
            )],
        );
    }

    #[test]
    fn a_party_with_a_skipped_line_still_has_its_block() {
        let text = "party 5\n  abbreviation \"D\"\n";
        check_errors(
            text,
            &[error(
                SyntaxErrorKind::UnexpectedToken,
                6,
                1,
                TokenKind::AtSign,
            )],
        );
        check(
            text,
            &expect![[r#"
            Document@0..27
              Party@0..27
                Word@0..5 "party"
                Whitespace@5..6 " "
                ErrorNode@6..7
                  Integer@6..7 "5"
                Newline@7..8 "\n"
                Whitespace@8..10 "  "
                Block@10..27
                  Indent@10..10 ""
                  StringField@10..27
                    Word@10..22 "abbreviation"
                    Whitespace@22..23 " "
                    String@23..26 "\"D\""
                    Newline@26..27 "\n"
                  Dedent@27..27 ""
        "#]],
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
