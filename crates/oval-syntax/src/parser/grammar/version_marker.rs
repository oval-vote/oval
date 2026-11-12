use crate::parser::Parser;
use crate::{SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn version_marker(&mut self) {
        self.value_field(SyntaxKind::VersionMarker, TokenKind::Integer);
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

    fn missing_version(offset: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingValue,
            offset,
            len: 0,
            expected: vec![Expected::Token(TokenKind::Integer)],
        }
    }

    #[test]
    fn a_complete_marker_has_no_errors() {
        check_errors("oval 1\n", &[]);
    }

    #[test]
    fn a_marker_without_a_version_is_missing_a_value() {
        check_errors("oval\n", &[missing_version(4)]);
    }

    #[test]
    fn a_missing_version_is_reported_at_the_end_of_the_line() {
        check_errors("oval   \n", &[missing_version(4)]);
        check_errors("oval  # version?\n", &[missing_version(4)]);
    }

    #[test]
    fn a_marker_without_a_version_at_the_end_of_input_is_missing_a_value() {
        check_errors("oval", &[missing_version(4)]);
    }

    #[test]
    fn a_version_marker_is_a_node() {
        check(
            "oval 1\n",
            &expect![[r#"
            Document@0..7
              VersionMarker@0..7
                Word@0..4 "oval"
                Whitespace@4..5 " "
                Integer@5..6 "1"
                Newline@6..7 "\n"
        "#]],
        );
    }

    #[test]
    fn ignored_tokens_before_the_marker_are_outside_it() {
        check(
            "# Franklin County\n\noval 1\n",
            &expect![[r##"
            Document@0..26
              Comment@0..17 "# Franklin County"
              BlankLine@17..18 "\n"
              BlankLine@18..19 "\n"
              VersionMarker@19..26
                Word@19..23 "oval"
                Whitespace@23..24 " "
                Integer@24..25 "1"
                Newline@25..26 "\n"
        "##]],
        );
    }

    #[test]
    fn the_rest_of_the_document_follows_the_marker() {
        check(
            "oval 1\nguide \"G\"\n",
            &expect![[r#"
                Document@0..17
                  VersionMarker@0..7
                    Word@0..4 "oval"
                    Whitespace@4..5 " "
                    Integer@5..6 "1"
                    Newline@6..7 "\n"
                  StringField@7..17
                    Word@7..12 "guide"
                    Whitespace@12..13 " "
                    String@13..16 "\"G\""
                    Newline@16..17 "\n"
            "#]],
        );
    }

    #[test]
    fn a_document_need_not_have_a_marker() {
        check(
            "guide \"G\"\n",
            &expect![[r#"
                Document@0..10
                  StringField@0..10
                    Word@0..5 "guide"
                    Whitespace@5..6 " "
                    String@6..9 "\"G\""
                    Newline@9..10 "\n"
            "#]],
        );
    }

    #[test]
    fn a_marker_without_a_version_is_still_a_marker() {
        check(
            "oval\n",
            &expect![[r#"
            Document@0..5
              VersionMarker@0..5
                Word@0..4 "oval"
                Newline@4..5 "\n"
        "#]],
        );
    }

    #[test]
    fn a_marker_at_the_end_of_input_ends_in_an_empty_newline() {
        check(
            "oval 1",
            &expect![[r#"
            Document@0..6
              VersionMarker@0..6
                Word@0..4 "oval"
                Whitespace@4..5 " "
                Integer@5..6 "1"
                Newline@6..6 ""
        "#]],
        );
    }

    #[test]
    fn only_the_first_entry_can_be_a_marker() {
        check(
            "guide \"G\"\noval 1\n",
            &expect![[r#"
                Document@0..17
                  StringField@0..10
                    Word@0..5 "guide"
                    Whitespace@5..6 " "
                    String@6..9 "\"G\""
                    Newline@9..10 "\n"
                  Word@10..14 "oval"
                  Whitespace@14..15 " "
                  Integer@15..16 "1"
                  Newline@16..17 "\n"
            "#]],
        );
    }

    #[test]
    fn a_marker_must_be_the_word_oval_exactly() {
        check(
            "Oval 1\n",
            &expect![[r#"
            Document@0..7
              Word@0..4 "Oval"
              Whitespace@4..5 " "
              Integer@5..6 "1"
              Newline@6..7 "\n"
        "#]],
        );
    }

    #[test]
    fn a_malformed_version_is_only_a_lexical_error() {
        let result = parse("oval 1x\n").unwrap();
        assert_eq!(result.syntax_errors, []);
        assert_eq!(result.lexical_errors.len(), 1);
        check(
            "oval 1x\n",
            &expect![[r#"
            Document@0..8
              VersionMarker@0..8
                Word@0..4 "oval"
                Whitespace@4..5 " "
                Error@5..7 "1x"
                Newline@7..8 "\n"
        "#]],
        );
    }

    #[test]
    fn a_version_that_is_not_an_integer_is_an_unexpected_token() {
        check_errors(
            "oval x\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedToken,
                offset: 5,
                len: 1,
                expected: vec![Expected::Token(TokenKind::Integer)],
            }],
        );
        check(
            "oval x\n",
            &expect![[r#"
            Document@0..7
              VersionMarker@0..7
                Word@0..4 "oval"
                Whitespace@4..5 " "
                ErrorNode@5..6
                  Word@5..6 "x"
                Newline@6..7 "\n"
        "#]],
        );
    }
}
