use crate::parser::Parser;
use crate::{Keyword, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn guide_field(&mut self) -> bool {
        match self.current_keyword() {
            Some(Keyword::Guide | Keyword::Election) => {
                self.value_field(SyntaxKind::StringField, TokenKind::String);
            }
            Some(Keyword::Date) => self.value_field(SyntaxKind::DateField, TokenKind::Date),
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
              Word@0..5 "Guide"
              Whitespace@5..6 " "
              String@6..9 "\"G\""
              Newline@9..10 "\n"
        "#]],
        );
    }
}
