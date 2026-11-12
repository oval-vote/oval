use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxKind, TokenKind};

const RECOMMEND_FIELDS: [Expected; 2] = [
    Expected::Keyword(Keyword::Justification),
    Expected::Keyword(Keyword::Note),
];

impl Parser<'_> {
    pub(super) fn contest_recommend(&mut self) {
        self.start_node(SyntaxKind::ContestRecommend);
        self.bump();
        if matches!(
            self.current_keyword(),
            Some(Keyword::NoPosition | Keyword::Abstain)
        ) {
            self.start_node(SyntaxKind::NoStance);
            self.bump();
            self.finish_node();
            self.end_of_line();
            if self.at(TokenKind::Indent) {
                self.block(&RECOMMEND_FIELDS, Self::recommend_field);
            }
        } else {
            if !self.at_line_end() {
                self.skip_line(vec![
                    Expected::Keyword(Keyword::NoPosition),
                    Expected::Keyword(Keyword::Abstain),
                    Expected::Token(TokenKind::Newline),
                ]);
            }
            self.end_of_line_with_block(&RECOMMEND_FIELDS, Self::recommend_field);
        }
        self.finish_node();
    }

    fn recommend_field(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };
        match keyword {
            Keyword::Justification | Keyword::Note => {
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

    fn in_contest(lines: &str) -> String {
        format!("ballot @a \"A\"\n  contest @m \"M\" single\n{lines}")
    }

    fn check_errors(text: &str, expected: &[SyntaxError]) {
        assert_eq!(parse(text).unwrap().syntax_errors, expected, "{text:?}");
    }

    fn recommend_child_nodes(text: &str) -> Vec<SyntaxKind> {
        let tree = parse(text).unwrap().syntax();
        let recommend = tree
            .descendants()
            .find(|node| node.kind() == SyntaxKind::ContestRecommend)
            .expect("the contest has a recommendation");
        recommend.children().map(|node| node.kind()).collect()
    }

    fn missing_block(offset: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingBlock,
            offset,
            len: 0,
            expected: vec![Expected::Token(TokenKind::Indent)],
        }
    }

    #[test]
    fn a_recommendation_without_a_stance_holds_its_fields() {
        let text = in_contest(
            "    recommend abstain\n      justification:\n        J.\n      note:\n        N.\n",
        );
        check_errors(&text, &[]);
        check(
            &text,
            &expect![[r#"
            Document@0..115
              Ballot@0..115
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "a"
                Whitespace@9..10 " "
                String@10..13 "\"A\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..115
                  Indent@16..16 ""
                  Contest@16..115
                    Word@16..23 "contest"
                    Whitespace@23..24 " "
                    Identifier@24..26
                      AtSign@24..25 "@"
                      Word@25..26 "m"
                    Whitespace@26..27 " "
                    String@27..30 "\"M\""
                    Whitespace@30..31 " "
                    ContestKind@31..37
                      Word@31..37 "single"
                    Newline@37..38 "\n"
                    Whitespace@38..42 "    "
                    Block@42..115
                      Indent@42..42 ""
                      ContestRecommend@42..115
                        Word@42..51 "recommend"
                        Whitespace@51..52 " "
                        NoStance@52..59
                          Word@52..59 "abstain"
                        Newline@59..60 "\n"
                        Whitespace@60..66 "      "
                        Block@66..115
                          Indent@66..66 ""
                          MarkdownField@66..92
                            Word@66..79 "justification"
                            Colon@79..80 ":"
                            Newline@80..81 "\n"
                            MarkdownBlock@81..92 "        J.\n"
                          Whitespace@92..98 "      "
                          MarkdownField@98..115
                            Word@98..102 "note"
                            Colon@102..103 ":"
                            Newline@103..104 "\n"
                            MarkdownBlock@104..115 "        N.\n"
                          Dedent@115..115 ""
                      Dedent@115..115 ""
                  Dedent@115..115 ""
        "#]],
        );
    }

    #[test]
    fn a_recommendation_without_a_stance_needs_no_block() {
        for stance in ["no-position", "abstain"] {
            let text = in_contest(&format!("    recommend {stance}\n"));
            check_errors(&text, &[]);
            assert_eq!(
                recommend_child_nodes(&text),
                [SyntaxKind::NoStance],
                "{stance}"
            );
        }
    }

    #[test]
    fn a_recommendation_with_stances_is_missing_its_block() {
        check_errors(&in_contest("    recommend\n"), &[missing_block(52)]);
    }

    #[test]
    fn a_word_that_is_not_a_stance_is_an_unexpected_token() {
        check_errors(
            &in_contest("    recommend yes\n"),
            &[
                SyntaxError {
                    kind: SyntaxErrorKind::UnexpectedToken,
                    offset: 52,
                    len: 3,
                    expected: vec![
                        Expected::Keyword(Keyword::NoPosition),
                        Expected::Keyword(Keyword::Abstain),
                        Expected::Token(TokenKind::Newline),
                    ],
                },
                missing_block(56),
            ],
        );
    }

    #[test]
    fn a_token_after_the_stance_is_an_unexpected_token() {
        check_errors(
            &in_contest("    recommend abstain now\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedToken,
                offset: 60,
                len: 3,
                expected: vec![Expected::Token(TokenKind::Newline)],
            }],
        );
    }

    #[test]
    fn a_repeated_recommendation_field_is_set_aside() {
        check_errors(
            &in_contest(
                "    recommend abstain\n      justification:\n        J.\n      justification:\n        K.\n",
            ),
            &[SyntaxError {
                kind: SyntaxErrorKind::RepeatedField,
                offset: 98,
                len: 26,
                expected: Vec::new(),
            }],
        );
    }

    #[test]
    fn a_contest_field_is_an_unexpected_entry_in_a_recommendation() {
        check_errors(
            &in_contest("    recommend abstain\n      subtitle \"S\"\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 66,
                len: 12,
                expected: vec![
                    Expected::Keyword(Keyword::Justification),
                    Expected::Keyword(Keyword::Note),
                ],
            }],
        );
    }

    #[test]
    fn a_requirement_before_a_recommendation_is_an_unexpected_entry() {
        check_errors(
            &in_contest("    required recommend abstain\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 42,
                len: 26,
                expected: vec![
                    Expected::Keyword(Keyword::Subtitle),
                    Expected::Keyword(Keyword::Tabulation),
                    Expected::Keyword(Keyword::Note),
                    Expected::Keyword(Keyword::Campaign),
                    Expected::Keyword(Keyword::Recommend),
                ],
            }],
        );
    }
}
