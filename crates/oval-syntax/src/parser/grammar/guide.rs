use crate::parser::Parser;
use crate::{Keyword, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn guide_field(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };
        match keyword {
            Keyword::Guide | Keyword::Election => self.field_once(seen, keyword, |p| {
                p.value_field(SyntaxKind::StringField, TokenKind::String);
            }),
            Keyword::Date => self.field_once(seen, keyword, |p| {
                p.value_field(SyntaxKind::DateField, TokenKind::Date);
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
    use crate::{Expected, LexicalErrorKind, SyntaxError, SyntaxErrorKind, TokenKind, parse};
    use expect_test::expect;

    #[test]
    fn a_guide_field_is_a_string_field() {
        check(
            "guide \"Franklin Voter Guide\"\n",
            &expect![[r#"
            Document@0..29
              StringField@0..29
                Word@0..5 "guide"
                Whitespace@5..6 " "
                String@6..28 "\"Franklin Voter Guide\""
                Newline@28..29 "\n"
        "#]],
        );
    }

    #[test]
    fn an_election_field_is_a_string_field() {
        check(
            "election \"General Election\"\n",
            &expect![[r#"
            Document@0..28
              StringField@0..28
                Word@0..8 "election"
                Whitespace@8..9 " "
                String@9..27 "\"General Election\""
                Newline@27..28 "\n"
        "#]],
        );
    }

    #[test]
    fn a_date_field_is_a_date_field() {
        check(
            "date 2026-11-03\n",
            &expect![[r#"
            Document@0..16
              DateField@0..16
                Word@0..4 "date"
                Whitespace@4..5 " "
                Date@5..15 "2026-11-03"
                Newline@15..16 "\n"
        "#]],
        );
    }

    #[test]
    fn guide_fields_follow_the_version_marker_in_any_order() {
        check(
            "oval 1\n\ndate 2026-11-03\n# The election\nelection \"E\"\nguide \"G\"\n",
            &expect![[r##"
                Document@0..62
                  VersionMarker@0..7
                    Word@0..4 "oval"
                    Whitespace@4..5 " "
                    Integer@5..6 "1"
                    Newline@6..7 "\n"
                  BlankLine@7..8 "\n"
                  DateField@8..24
                    Word@8..12 "date"
                    Whitespace@12..13 " "
                    Date@13..23 "2026-11-03"
                    Newline@23..24 "\n"
                  Comment@24..38 "# The election"
                  BlankLine@38..39 "\n"
                  StringField@39..52
                    Word@39..47 "election"
                    Whitespace@47..48 " "
                    String@48..51 "\"E\""
                    Newline@51..52 "\n"
                  StringField@52..62
                    Word@52..57 "guide"
                    Whitespace@57..58 " "
                    String@58..61 "\"G\""
                    Newline@61..62 "\n"
            "##]],
        );
    }

    #[test]
    fn a_field_without_a_value_is_missing_a_value() {
        let result = parse("guide\n").unwrap();
        assert_eq!(
            result.syntax_errors,
            [SyntaxError {
                kind: SyntaxErrorKind::MissingValue,
                offset: 5,
                len: 0,
                expected: vec![Expected::Token(TokenKind::String)],
            }]
        );
        check(
            "guide\n",
            &expect![[r#"
            Document@0..6
              StringField@0..6
                Word@0..5 "guide"
                Newline@5..6 "\n"
        "#]],
        );
    }

    #[test]
    fn a_malformed_date_is_only_a_lexical_error() {
        let result = parse("date 2026-11-5\n").unwrap();
        assert_eq!(result.syntax_errors, []);
        assert_eq!(
            result
                .lexical_errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            [LexicalErrorKind::MalformedNumber]
        );
        check(
            "date 2026-11-5\n",
            &expect![[r#"
            Document@0..15
              DateField@0..15
                Word@0..4 "date"
                Whitespace@4..5 " "
                Error@5..14 "2026-11-5"
                Newline@14..15 "\n"
        "#]],
        );
    }

    #[test]
    fn keywords_are_case_sensitive() {
        check(
            "Guide \"G\"\n",
            &expect![[r#"
                Document@0..10
                  ErrorNode@0..10
                    Word@0..5 "Guide"
                    Whitespace@5..6 " "
                    String@6..9 "\"G\""
                    Newline@9..10 "\n"
            "#]],
        );
    }

    fn unexpected(offset: u32, len: u32, expected: TokenKind) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedToken,
            offset,
            len,
            expected: vec![Expected::Token(expected)],
        }
    }

    #[test]
    fn a_value_of_the_wrong_kind_is_discarded() {
        let text = "date \"November 3, 2026\"\n";
        assert_eq!(
            parse(text).unwrap().syntax_errors,
            [unexpected(5, 18, TokenKind::Date)]
        );
        check(
            text,
            &expect![[r#"
            Document@0..24
              DateField@0..24
                Word@0..4 "date"
                Whitespace@4..5 " "
                ErrorNode@5..23
                  String@5..23 "\"November 3, 2026\""
                Newline@23..24 "\n"
        "#]],
        );
    }

    #[test]
    fn a_token_after_a_complete_field_is_discarded() {
        let text = "guide \"G\" extra\n";
        assert_eq!(
            parse(text).unwrap().syntax_errors,
            [unexpected(10, 5, TokenKind::Newline)]
        );
        check(
            text,
            &expect![[r#"
            Document@0..16
              StringField@0..16
                Word@0..5 "guide"
                Whitespace@5..6 " "
                String@6..9 "\"G\""
                Whitespace@9..10 " "
                ErrorNode@10..15
                  Word@10..15 "extra"
                Newline@15..16 "\n"
        "#]],
        );
    }

    #[test]
    fn a_second_value_is_discarded_but_not_a_comment_after_it() {
        let text = "guide \"Vote for one\" \"Pick one\"  # which wording?\n";
        assert_eq!(
            parse(text).unwrap().syntax_errors,
            [unexpected(21, 10, TokenKind::Newline)]
        );
        check(
            text,
            &expect![[r##"
            Document@0..50
              StringField@0..50
                Word@0..5 "guide"
                Whitespace@5..6 " "
                String@6..20 "\"Vote for one\""
                Whitespace@20..21 " "
                ErrorNode@21..31
                  String@21..31 "\"Pick one\""
                Whitespace@31..33 "  "
                Comment@33..49 "# which wording?"
                Newline@49..50 "\n"
        "##]],
        );
    }

    #[test]
    fn the_rest_of_the_line_is_discarded_together() {
        assert_eq!(
            parse("guide 1 2 3\n").unwrap().syntax_errors,
            [unexpected(6, 5, TokenKind::String)]
        );
    }

    #[test]
    fn an_unexpected_token_beside_an_error_token_is_still_reported() {
        let result = parse("date ~ \"x\"\n").unwrap();
        assert_eq!(result.syntax_errors, [unexpected(7, 3, TokenKind::Date)]);
        assert_eq!(result.lexical_errors.len(), 1);
    }

    #[test]
    fn parsing_continues_on_the_line_after_a_skipped_one() {
        let text = "guide 1\ndate 2026-11-03\n";
        assert_eq!(
            parse(text).unwrap().syntax_errors,
            [unexpected(6, 1, TokenKind::String)]
        );
        check(
            text,
            &expect![[r#"
            Document@0..24
              StringField@0..8
                Word@0..5 "guide"
                Whitespace@5..6 " "
                ErrorNode@6..7
                  Integer@6..7 "1"
                Newline@7..8 "\n"
              DateField@8..24
                Word@8..12 "date"
                Whitespace@12..13 " "
                Date@13..23 "2026-11-03"
                Newline@23..24 "\n"
        "#]],
        );
    }

    #[test]
    fn the_specification_example_parses_completely() {
        let text = "oval 1\n\nguide \"Franklin Voter Guide\"\nelection \"November 3, 2026 General Election\"\ndate 2026-11-03\n\nnote:\n  Written by the Franklin Civic Alliance.\n";
        let result = parse(text).unwrap();
        assert_eq!(result.lexical_errors, []);
        assert_eq!(result.syntax_errors, []);
        check(
            text,
            &expect![[r#"
                Document@0..147
                  VersionMarker@0..7
                    Word@0..4 "oval"
                    Whitespace@4..5 " "
                    Integer@5..6 "1"
                    Newline@6..7 "\n"
                  BlankLine@7..8 "\n"
                  StringField@8..37
                    Word@8..13 "guide"
                    Whitespace@13..14 " "
                    String@14..36 "\"Franklin Voter Guide\""
                    Newline@36..37 "\n"
                  StringField@37..82
                    Word@37..45 "election"
                    Whitespace@45..46 " "
                    String@46..81 "\"November 3, 2026 General Election\""
                    Newline@81..82 "\n"
                  DateField@82..98
                    Word@82..86 "date"
                    Whitespace@86..87 " "
                    Date@87..97 "2026-11-03"
                    Newline@97..98 "\n"
                  BlankLine@98..99 "\n"
                  MarkdownField@99..147
                    Word@99..103 "note"
                    Colon@103..104 ":"
                    Newline@104..105 "\n"
                    MarkdownBlock@105..147 "  Written by the Franklin Civic Alliance.\n"
            "#]],
        );
    }

    fn repeated(offset: u32, len: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::RepeatedField,
            offset,
            len,
            expected: Vec::new(),
        }
    }

    #[test]
    fn a_repeated_field_is_set_aside() {
        let text = "guide \"A\"\nguide \"B\"\n";
        assert_eq!(parse(text).unwrap().syntax_errors, [repeated(10, 9)]);
        check(
            text,
            &expect![[r#"
            Document@0..20
              StringField@0..10
                Word@0..5 "guide"
                Whitespace@5..6 " "
                String@6..9 "\"A\""
                Newline@9..10 "\n"
              ErrorNode@10..20
                StringField@10..20
                  Word@10..15 "guide"
                  Whitespace@15..16 " "
                  String@16..19 "\"B\""
                  Newline@19..20 "\n"
        "#]],
        );
    }

    #[test]
    fn every_repeat_after_the_first_is_set_aside() {
        assert_eq!(
            parse("guide \"A\"\nguide \"B\"\nguide \"C\"\n")
                .unwrap()
                .syntax_errors,
            [repeated(10, 9), repeated(20, 9)]
        );
    }

    #[test]
    fn errors_inside_a_repeated_field_follow_it() {
        assert_eq!(
            parse("date 2026-11-03\ndate \"x\"\n")
                .unwrap()
                .syntax_errors,
            [repeated(16, 8), unexpected(21, 3, TokenKind::Date)]
        );
    }

    #[test]
    fn a_repeated_markdown_field_is_set_aside_with_its_body() {
        let text = "note:\n  A.\nnote:\n  B.\n";
        assert_eq!(parse(text).unwrap().syntax_errors, [repeated(11, 11)]);
        check(
            text,
            &expect![[r#"
            Document@0..22
              MarkdownField@0..11
                Word@0..4 "note"
                Colon@4..5 ":"
                Newline@5..6 "\n"
                MarkdownBlock@6..11 "  A.\n"
              ErrorNode@11..22
                MarkdownField@11..22
                  Word@11..15 "note"
                  Colon@15..16 ":"
                  Newline@16..17 "\n"
                  MarkdownBlock@17..22 "  B.\n"
        "#]],
        );
    }

    #[test]
    fn a_repeated_field_does_not_span_a_comment_after_it() {
        assert_eq!(
            parse("guide \"A\"\nguide \"B\"  # again\n")
                .unwrap()
                .syntax_errors,
            [repeated(10, 9)]
        );
    }

    #[test]
    fn different_fields_are_not_repeats() {
        assert_eq!(
            parse("guide \"A\"\nelection \"B\"\ndate 2026-11-03\nnote:\n  C.\n")
                .unwrap()
                .syntax_errors,
            []
        );
    }
}
