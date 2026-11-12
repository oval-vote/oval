use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const MEASURE_ENTRIES: [Expected; 8] = [
    Expected::Keyword(Keyword::Subtitle),
    Expected::Keyword(Keyword::Description),
    Expected::Keyword(Keyword::IfYes),
    Expected::Keyword(Keyword::IfNo),
    Expected::Keyword(Keyword::Placement),
    Expected::Keyword(Keyword::Tabulation),
    Expected::Keyword(Keyword::FiscalImpact),
    Expected::Keyword(Keyword::Note),
];

impl Parser<'_> {
    pub(super) fn measure(&mut self) {
        self.start_node(SyntaxKind::Measure);
        self.modifiers();
        self.bump();
        self.identifier();
        self.expect(TokenKind::String, SyntaxErrorKind::MissingName);
        self.end_of_line_with_block(&MEASURE_ENTRIES, Self::measure_entry);
        self.finish_node();
    }

    fn measure_entry(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };
        match keyword {
            Keyword::Subtitle
            | Keyword::Placement
            | Keyword::Tabulation
            | Keyword::FiscalImpact => {
                self.field_once(seen, keyword, |p| {
                    p.value_field(SyntaxKind::StringField, TokenKind::String);
                });
            }
            Keyword::Description | Keyword::IfYes | Keyword::IfNo | Keyword::Note => {
                self.field_once(seen, keyword, Self::markdown_field);
            }
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::tests::check;
    use crate::{Expected, Keyword, SyntaxError, SyntaxErrorKind, SyntaxKind, TokenKind, parse};
    use expect_test::expect;

    fn check_errors(text: &str, expected: &[SyntaxError]) {
        assert_eq!(parse(text).unwrap().syntax_errors, expected, "{text:?}");
    }

    fn measure_entries() -> Vec<Expected> {
        vec![
            Expected::Keyword(Keyword::Subtitle),
            Expected::Keyword(Keyword::Description),
            Expected::Keyword(Keyword::IfYes),
            Expected::Keyword(Keyword::IfNo),
            Expected::Keyword(Keyword::Placement),
            Expected::Keyword(Keyword::Tabulation),
            Expected::Keyword(Keyword::FiscalImpact),
            Expected::Keyword(Keyword::Note),
        ]
    }

    #[test]
    fn a_measure_holds_its_fields() {
        let text = "ballot @a \"A\"\n  required measure @prop1 \"Proposition 1\"\n    subtitle \"Bond\"\n    description:\n      D.\n    fiscal_impact \"F\"\n";
        check_errors(text, &[]);
        check(
            text,
            &expect![[r#"
            Document@0..124
              Ballot@0..124
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "a"
                Whitespace@9..10 " "
                String@10..13 "\"A\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..124
                  Indent@16..16 ""
                  Measure@16..124
                    Requirement@16..24
                      Word@16..24 "required"
                    Whitespace@24..25 " "
                    Word@25..32 "measure"
                    Whitespace@32..33 " "
                    Identifier@33..39
                      AtSign@33..34 "@"
                      Word@34..39 "prop1"
                    Whitespace@39..40 " "
                    String@40..55 "\"Proposition 1\""
                    Newline@55..56 "\n"
                    Whitespace@56..60 "    "
                    Block@60..124
                      Indent@60..60 ""
                      StringField@60..76
                        Word@60..68 "subtitle"
                        Whitespace@68..69 " "
                        String@69..75 "\"Bond\""
                        Newline@75..76 "\n"
                      Whitespace@76..80 "    "
                      MarkdownField@80..102
                        Word@80..91 "description"
                        Colon@91..92 ":"
                        Newline@92..93 "\n"
                        MarkdownBlock@93..102 "      D.\n"
                      Whitespace@102..106 "    "
                      StringField@106..124
                        Word@106..119 "fiscal_impact"
                        Whitespace@119..120 " "
                        String@120..123 "\"F\""
                        Newline@123..124 "\n"
                      Dedent@124..124 ""
                  Dedent@124..124 ""
        "#]],
        );
    }

    #[test]
    fn a_measure_takes_every_measure_field() {
        check_errors(
            "ballot @a \"A\"\n  measure @p \"P\"\n    subtitle \"S\"\n    description:\n      D.\n    if_yes:\n      Y.\n    if_no:\n      N.\n    placement \"P\"\n    tabulation \"T\"\n    fiscal_impact \"F\"\n    note:\n      N.\n",
            &[],
        );
    }

    #[test]
    fn a_section_holds_measures() {
        let text = "ballot @a \"A\"\n  section \"S\"\n    required measure @p \"P\"\n      subtitle \"T\"\n";
        check_errors(text, &[]);
        let tree = parse(text).unwrap().syntax();
        assert!(
            tree.descendants()
                .any(|node| node.kind() == SyntaxKind::Measure)
        );
    }

    #[test]
    fn a_measure_without_a_block_is_missing_its_block() {
        check_errors(
            "ballot @a \"A\"\n  measure @p \"P\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::MissingBlock,
                offset: 31,
                len: 0,
                expected: vec![Expected::Token(TokenKind::Indent)],
            }],
        );
    }

    #[test]
    fn a_measure_without_a_name_is_missing_its_name() {
        check_errors(
            "ballot @a \"A\"\n  measure @p\n    subtitle \"S\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::MissingName,
                offset: 26,
                len: 0,
                expected: vec![Expected::Token(TokenKind::String)],
            }],
        );
    }

    #[test]
    fn an_extensibility_before_a_measure_is_an_unexpected_entry() {
        check_errors(
            "ballot @a \"A\"\n  extensible measure @p \"P\"\n    subtitle \"S\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 16,
                len: 42,
                expected: vec![
                    Expected::Keyword(Keyword::Eligibility),
                    Expected::Keyword(Keyword::Note),
                    Expected::Keyword(Keyword::Section),
                    Expected::Keyword(Keyword::Contest),
                    Expected::Keyword(Keyword::Measure),
                    Expected::Keyword(Keyword::Ballot),
                ],
            }],
        );
    }

    #[test]
    fn a_campaign_is_an_unexpected_entry_in_a_measure() {
        check_errors(
            "ballot @a \"A\"\n  measure @p \"P\"\n    campaign @c\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 35,
                len: 11,
                expected: measure_entries(),
            }],
        );
    }

    #[test]
    fn a_repeated_measure_field_is_set_aside() {
        check_errors(
            "ballot @a \"A\"\n  measure @p \"P\"\n    subtitle \"S\"\n    subtitle \"T\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::RepeatedField,
                offset: 52,
                len: 12,
                expected: Vec::new(),
            }],
        );
    }
}
