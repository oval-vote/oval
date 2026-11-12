use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const STANCE_ENTRIES: [Expected; 4] = [
    Expected::Keyword(Keyword::Support),
    Expected::Keyword(Keyword::Oppose),
    Expected::Keyword(Keyword::Justification),
    Expected::Keyword(Keyword::Note),
];

const RECOMMEND_FIELDS: [Expected; 2] = [
    Expected::Keyword(Keyword::Justification),
    Expected::Keyword(Keyword::Note),
];

impl Parser<'_> {
    pub(super) fn contest_recommend(&mut self) {
        self.start_node(SyntaxKind::ContestRecommend);
        let keyword_start = self.offset;
        self.bump();
        let keyword_end = self.offset;
        let index = self.errors.len();
        if matches!(
            self.current_keyword(),
            Some(Keyword::NoPosition | Keyword::Abstain)
        ) {
            self.start_node(SyntaxKind::NoStance);
            self.bump();
            self.finish_node();
            self.end_of_line();
            if self.at(TokenKind::Indent) {
                self.block(&RECOMMEND_FIELDS, Self::no_stance_entry);
            }
        } else {
            if !self.at_line_end() {
                self.skip_line(vec![
                    Expected::Keyword(Keyword::NoPosition),
                    Expected::Keyword(Keyword::Abstain),
                    Expected::Token(TokenKind::Newline),
                ]);
            }
            self.end_of_line();
            if self.at(TokenKind::Indent) {
                let mut has_stance = false;
                self.open_block();
                self.entries(&STANCE_ENTRIES, |p, seen| {
                    p.stance_entry(seen, &mut has_stance)
                });
                if !has_stance && !self.error_token_ahead() {
                    self.insert_error(
                        index,
                        SyntaxErrorKind::EmptyRecommendation,
                        keyword_start,
                        keyword_end,
                        vec![
                            Expected::Keyword(Keyword::Support),
                            Expected::Keyword(Keyword::Oppose),
                        ],
                    );
                }
                self.close_block();
            } else {
                self.missing_block();
            }
        }
        self.finish_node();
    }

    fn stance_entry(&mut self, seen: &mut Vec<Keyword>, has_stance: &mut bool) -> bool {
        let Some(kind) = self.stance_kind() else {
            return self.recommend_field(seen);
        };
        self.stance(kind);
        *has_stance = true;
        true
    }

    fn no_stance_entry(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let Some(kind) = self.stance_kind() else {
            return self.recommend_field(seen);
        };
        self.set_aside(SyntaxErrorKind::UnexpectedStance, |p| p.stance(kind));
        true
    }

    fn stance_kind(&self) -> Option<SyntaxKind> {
        match self.current_keyword()? {
            Keyword::Support => Some(SyntaxKind::Support),
            Keyword::Oppose => Some(SyntaxKind::Oppose),
            _ => None,
        }
    }

    fn stance(&mut self, kind: SyntaxKind) {
        self.start_node(kind);
        self.bump();
        self.choice();
        while self.at(TokenKind::AtSign) || self.current_keyword() == Some(Keyword::WriteIn) {
            self.choice();
        }
        if !self.at_line_end() {
            self.skip_line(vec![
                Expected::Token(TokenKind::AtSign),
                Expected::Keyword(Keyword::WriteIn),
                Expected::Token(TokenKind::Newline),
            ]);
        }
        self.end_of_line();
        self.finish_node();
    }

    fn choice(&mut self) {
        if self.current_keyword() == Some(Keyword::WriteIn) {
            self.start_node(SyntaxKind::WriteIn);
            self.bump();
            self.eat(TokenKind::String);
            self.finish_node();
        } else {
            self.identifier_expecting(vec![
                Expected::Token(TokenKind::AtSign),
                Expected::Keyword(Keyword::WriteIn),
            ]);
        }
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

    fn stance_child_nodes(text: &str) -> Vec<SyntaxKind> {
        let tree = parse(text).unwrap().syntax();
        let stance = tree
            .descendants()
            .find(|node| matches!(node.kind(), SyntaxKind::Support | SyntaxKind::Oppose))
            .expect("the recommendation has a stance");
        stance.children().map(|node| node.kind()).collect()
    }

    fn unexpected_token(offset: u32, len: u32, newline: bool) -> SyntaxError {
        let mut expected = vec![
            Expected::Token(TokenKind::AtSign),
            Expected::Keyword(Keyword::WriteIn),
        ];
        if newline {
            expected.push(Expected::Token(TokenKind::Newline));
        }
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedToken,
            offset,
            len,
            expected,
        }
    }

    #[test]
    fn stances_are_lines_in_the_block() {
        let text = in_contest(
            "    recommend\n      support @aokafor\n      support @bsantos write-in \"Hiro Eriksen\"\n      oppose @cfox write-in\n      justification:\n        J.\n",
        );
        check_errors(&text, &[]);
        check(
            &text,
            &expect![[r#"
            Document@0..182
              Ballot@0..182
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "a"
                Whitespace@9..10 " "
                String@10..13 "\"A\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..182
                  Indent@16..16 ""
                  Contest@16..182
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
                    Block@42..182
                      Indent@42..42 ""
                      ContestRecommend@42..182
                        Word@42..51 "recommend"
                        Newline@51..52 "\n"
                        Whitespace@52..58 "      "
                        Block@58..182
                          Indent@58..58 ""
                          Support@58..75
                            Word@58..65 "support"
                            Whitespace@65..66 " "
                            Identifier@66..74
                              AtSign@66..67 "@"
                              Word@67..74 "aokafor"
                            Newline@74..75 "\n"
                          Whitespace@75..81 "      "
                          Support@81..122
                            Word@81..88 "support"
                            Whitespace@88..89 " "
                            Identifier@89..97
                              AtSign@89..90 "@"
                              Word@90..97 "bsantos"
                            Whitespace@97..98 " "
                            WriteIn@98..121
                              Word@98..106 "write-in"
                              Whitespace@106..107 " "
                              String@107..121 "\"Hiro Eriksen\""
                            Newline@121..122 "\n"
                          Whitespace@122..128 "      "
                          Oppose@128..150
                            Word@128..134 "oppose"
                            Whitespace@134..135 " "
                            Identifier@135..140
                              AtSign@135..136 "@"
                              Word@136..140 "cfox"
                            Whitespace@140..141 " "
                            WriteIn@141..149
                              Word@141..149 "write-in"
                            Newline@149..150 "\n"
                          Whitespace@150..156 "      "
                          MarkdownField@156..182
                            Word@156..169 "justification"
                            Colon@169..170 ":"
                            Newline@170..171 "\n"
                            MarkdownBlock@171..182 "        J.\n"
                          Dedent@182..182 ""
                      Dedent@182..182 ""
                  Dedent@182..182 ""
        "#]],
        );
    }

    #[test]
    fn a_stance_holds_references_and_write_ins() {
        let text = in_contest("    recommend\n      support @a @b write-in \"X\" write-in\n");
        check_errors(&text, &[]);
        assert_eq!(
            stance_child_nodes(&text),
            [
                SyntaxKind::Identifier,
                SyntaxKind::Identifier,
                SyntaxKind::WriteIn,
                SyntaxKind::WriteIn
            ]
        );
    }

    #[test]
    fn stances_and_fields_are_in_any_order() {
        check_errors(
            &in_contest(
                "    recommend\n      justification:\n        J.\n      oppose @c\n      support @a\n      oppose @d\n",
            ),
            &[],
        );
    }

    #[test]
    fn a_token_after_a_choice_is_an_unexpected_token() {
        check_errors(
            &in_contest("    recommend\n      support @a 5\n"),
            &[unexpected_token(69, 1, true)],
        );
        check_errors(
            &in_contest("    recommend\n      support @a \"A\"\n"),
            &[unexpected_token(69, 3, true)],
        );
    }

    #[test]
    fn a_token_in_place_of_the_first_choice_is_an_unexpected_token() {
        check_errors(
            &in_contest("    recommend\n      oppose 5\n"),
            &[unexpected_token(65, 1, false)],
        );
    }

    #[test]
    fn a_block_below_a_stance_is_unexpected() {
        check_errors(
            &in_contest("    recommend\n      support @a\n        justification:\n          J.\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedBlock,
                offset: 77,
                len: 28,
                expected: Vec::new(),
            }],
        );
    }

    #[test]
    fn a_contest_field_is_an_unexpected_entry_among_stances() {
        check_errors(
            &in_contest("    recommend\n      support @a\n      subtitle \"S\"\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 75,
                len: 12,
                expected: vec![
                    Expected::Keyword(Keyword::Support),
                    Expected::Keyword(Keyword::Oppose),
                    Expected::Keyword(Keyword::Justification),
                    Expected::Keyword(Keyword::Note),
                ],
            }],
        );
    }

    fn missing_identifier(offset: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingIdentifier,
            offset,
            len: 0,
            expected: vec![
                Expected::Token(TokenKind::AtSign),
                Expected::Keyword(Keyword::WriteIn),
            ],
        }
    }

    #[test]
    fn a_stance_without_a_choice_is_missing_its_identifier() {
        let text = in_contest("    recommend\n      support\n");
        check_errors(&text, &[missing_identifier(65)]);
        assert_eq!(stance_child_nodes(&text), []);
    }

    #[test]
    fn a_name_in_place_of_the_first_choice_is_missing_its_identifier_and_unexpected() {
        let text = in_contest("    recommend\n      oppose \"Jane Doe\"\n");
        check_errors(
            &text,
            &[missing_identifier(65), unexpected_token(65, 10, true)],
        );
        assert_eq!(stance_child_nodes(&text), [SyntaxKind::ErrorNode]);
    }

    #[test]
    fn choices_after_a_name_in_place_of_the_first_choice_are_skipped() {
        check_errors(
            &in_contest("    recommend\n      support \"Jane\" @b\n"),
            &[missing_identifier(66), unexpected_token(66, 9, true)],
        );
    }

    #[test]
    fn a_missing_choice_is_not_reported_where_an_error_token_is() {
        let result = parse(&in_contest("    recommend\n      support ~\n")).unwrap();
        assert_eq!(result.lexical_errors.len(), 1);
        assert_eq!(result.syntax_errors, []);
    }

    fn empty_recommendation() -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::EmptyRecommendation,
            offset: 42,
            len: 9,
            expected: vec![
                Expected::Keyword(Keyword::Support),
                Expected::Keyword(Keyword::Oppose),
            ],
        }
    }

    #[test]
    fn a_block_without_a_stance_is_an_empty_recommendation() {
        let text = in_contest("    recommend\n      justification:\n        J.\n");
        check_errors(&text, &[empty_recommendation()]);
        assert_eq!(recommend_child_nodes(&text), [SyntaxKind::Block]);
    }

    #[test]
    fn an_empty_recommendation_comes_before_errors_in_its_block() {
        check_errors(
            &in_contest("    recommend\n      subtitle \"S\"\n"),
            &[
                empty_recommendation(),
                SyntaxError {
                    kind: SyntaxErrorKind::UnexpectedEntry,
                    offset: 58,
                    len: 12,
                    expected: vec![
                        Expected::Keyword(Keyword::Support),
                        Expected::Keyword(Keyword::Oppose),
                        Expected::Keyword(Keyword::Justification),
                        Expected::Keyword(Keyword::Note),
                    ],
                },
            ],
        );
    }

    #[test]
    fn an_empty_recommendation_comes_before_errors_on_its_line() {
        check_errors(
            &in_contest("    recommend yes\n      justification:\n        J.\n"),
            &[
                empty_recommendation(),
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
            ],
        );
    }

    #[test]
    fn an_oppose_line_is_a_stance() {
        check_errors(&in_contest("    recommend\n      oppose @c\n"), &[]);
    }

    #[test]
    fn a_stance_line_with_an_error_is_still_a_stance() {
        check_errors(
            &in_contest("    recommend\n      support 5\n"),
            &[unexpected_token(66, 1, false)],
        );
    }

    #[test]
    fn an_empty_recommendation_is_not_reported_where_an_error_token_ends_the_block() {
        for lines in [
            "    recommend\n      justification:\n        J.\n      ~\n",
            "    recommend\n      ~\n",
        ] {
            let result = parse(&in_contest(lines)).unwrap();
            assert_eq!(result.lexical_errors.len(), 1, "{lines:?}");
            assert_eq!(result.syntax_errors, [], "{lines:?}");
        }
    }

    #[test]
    fn an_error_token_earlier_in_the_block_does_not_hide_an_empty_recommendation() {
        let result = parse(&in_contest(
            "    recommend\n      ~\n      justification:\n        J.\n",
        ))
        .unwrap();
        assert_eq!(result.lexical_errors.len(), 1);
        assert_eq!(result.syntax_errors, [empty_recommendation()]);
    }

    fn unexpected_stance(offset: u32, len: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedStance,
            offset,
            len,
            expected: Vec::new(),
        }
    }

    #[test]
    fn a_stance_under_a_recommendation_without_one_is_set_aside() {
        let text = in_contest(
            "    recommend abstain\n      support @bsantos\n      justification:\n        J.\n",
        );
        check_errors(&text, &[unexpected_stance(66, 16)]);
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
                          ErrorNode@66..83
                            Support@66..83
                              Word@66..73 "support"
                              Whitespace@73..74 " "
                              Identifier@74..82
                                AtSign@74..75 "@"
                                Word@75..82 "bsantos"
                              Newline@82..83 "\n"
                          Whitespace@83..89 "      "
                          MarkdownField@89..115
                            Word@89..102 "justification"
                            Colon@102..103 ":"
                            Newline@103..104 "\n"
                            MarkdownBlock@104..115 "        J.\n"
                          Dedent@115..115 ""
                      Dedent@115..115 ""
                  Dedent@115..115 ""
        "#]],
        );
    }

    #[test]
    fn each_unexpected_stance_is_reported() {
        check_errors(
            &in_contest("    recommend no-position\n      oppose @c\n      support @a\n"),
            &[unexpected_stance(70, 9), unexpected_stance(86, 10)],
        );
    }

    #[test]
    fn errors_inside_an_unexpected_stance_follow_it() {
        check_errors(
            &in_contest("    recommend abstain\n      support \"Jane\"\n"),
            &[
                unexpected_stance(66, 14),
                missing_identifier(74),
                unexpected_token(74, 6, true),
            ],
        );
    }
}
