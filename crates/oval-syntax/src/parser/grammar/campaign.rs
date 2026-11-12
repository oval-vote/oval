use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const CAMPAIGN_ENTRIES: [Expected; 4] = [
    Expected::Keyword(Keyword::Website),
    Expected::Keyword(Keyword::Note),
    Expected::Keyword(Keyword::Designation),
    Expected::Keyword(Keyword::Party),
];

impl Parser<'_> {
    pub(super) fn campaign(&mut self) {
        self.start_node(SyntaxKind::Campaign);
        self.bump();
        self.flags();
        if self.at(TokenKind::AtSign) {
            self.member();
            self.end_of_line();
            if self.at(TokenKind::Indent) {
                self.block(&CAMPAIGN_ENTRIES, Self::campaign_entry);
            }
        } else {
            if !self.at_line_end() {
                self.skip_line(vec![
                    Expected::Keyword(Keyword::Incumbent),
                    Expected::Keyword(Keyword::Withdrawn),
                    Expected::Token(TokenKind::AtSign),
                    Expected::Token(TokenKind::Newline),
                ]);
            }
            self.end_of_line_with_block(&CAMPAIGN_ENTRIES, Self::campaign_entry);
        }
        self.finish_node();
    }

    fn flags(&mut self) {
        while matches!(
            self.current_keyword(),
            Some(Keyword::Incumbent | Keyword::Withdrawn)
        ) {
            self.start_node(SyntaxKind::Flag);
            self.bump();
            self.finish_node();
        }
    }

    fn member(&mut self) {
        self.start_node(SyntaxKind::Member);
        self.identifier();
        let mut expected = Vec::new();
        if !self.eat(TokenKind::String) {
            expected.push(Expected::Token(TokenKind::String));
        }
        if self.current_keyword() == Some(Keyword::As) {
            self.member_role();
        } else if !self.at_line_end() {
            expected.push(Expected::Keyword(Keyword::As));
            expected.push(Expected::Token(TokenKind::Newline));
            self.skip_line(expected);
        }
        self.finish_node();
    }

    fn member_role(&mut self) {
        self.start_node(SyntaxKind::MemberRole);
        self.bump();
        self.expect(TokenKind::String, SyntaxErrorKind::MissingValue);
        self.finish_node();
    }

    fn campaign_entry(&mut self, seen: &mut Vec<Keyword>) -> bool {
        if self.current_keyword() == Some(Keyword::Website) {
            self.field_once(seen, Keyword::Website, |p| {
                p.value_field(SyntaxKind::UrlField, TokenKind::Url);
            });
            return true;
        }
        self.candidate_field(seen)
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

    fn child_nodes(text: &str, kind: SyntaxKind) -> Vec<SyntaxKind> {
        let tree = parse(text).unwrap().syntax();
        let node = tree
            .descendants()
            .find(|node| node.kind() == kind)
            .expect("the tree has the node");
        node.children().map(|node| node.kind()).collect()
    }

    fn unexpected_token(offset: u32, len: u32, expected: Vec<Expected>) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedToken,
            offset,
            len,
            expected,
        }
    }

    #[test]
    fn a_campaign_holds_its_flags_member_and_fields() {
        let text = in_contest(
            "    campaign incumbent @jlee \"Jordan Lee\" as \"Governor\"\n      designation \"Governor\"\n      website https://lee.example\n",
        );
        check_errors(&text, &[]);
        check(
            &text,
            &expect![[r#"
            Document@0..157
              Ballot@0..157
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "a"
                Whitespace@9..10 " "
                String@10..13 "\"A\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..157
                  Indent@16..16 ""
                  Contest@16..157
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
                    Block@42..157
                      Indent@42..42 ""
                      Campaign@42..157
                        Word@42..50 "campaign"
                        Whitespace@50..51 " "
                        Flag@51..60
                          Word@51..60 "incumbent"
                        Whitespace@60..61 " "
                        Member@61..93
                          Identifier@61..66
                            AtSign@61..62 "@"
                            Word@62..66 "jlee"
                          Whitespace@66..67 " "
                          String@67..79 "\"Jordan Lee\""
                          Whitespace@79..80 " "
                          MemberRole@80..93
                            Word@80..82 "as"
                            Whitespace@82..83 " "
                            String@83..93 "\"Governor\""
                        Newline@93..94 "\n"
                        Whitespace@94..100 "      "
                        Block@100..157
                          Indent@100..100 ""
                          StringField@100..123
                            Word@100..111 "designation"
                            Whitespace@111..112 " "
                            String@112..122 "\"Governor\""
                            Newline@122..123 "\n"
                          Whitespace@123..129 "      "
                          UrlField@129..157
                            Word@129..136 "website"
                            Whitespace@136..137 " "
                            Url@137..156 "https://lee.example"
                            Newline@156..157 "\n"
                          Dedent@157..157 ""
                      Dedent@157..157 ""
                  Dedent@157..157 ""
        "#]],
        );
    }

    #[test]
    fn a_member_can_refer_to_a_candidate() {
        let text = in_contest("    campaign @jlee\n");
        check_errors(&text, &[]);
        assert_eq!(
            child_nodes(&text, SyntaxKind::Member),
            [SyntaxKind::Identifier]
        );
    }

    #[test]
    fn a_reference_can_have_a_role() {
        let text = in_contest("    campaign @jlee as \"Governor\"\n");
        check_errors(&text, &[]);
        assert_eq!(
            child_nodes(&text, SyntaxKind::Member),
            [SyntaxKind::Identifier, SyntaxKind::MemberRole]
        );
    }

    #[test]
    fn flags_are_in_any_order() {
        let text = in_contest("    campaign withdrawn incumbent @jlee\n");
        check_errors(&text, &[]);
        assert_eq!(
            child_nodes(&text, SyntaxKind::Campaign),
            [SyntaxKind::Flag, SyntaxKind::Flag, SyntaxKind::Member]
        );
    }

    #[test]
    fn a_contest_holds_several_campaigns() {
        let text = in_contest("    campaign @a\n    campaign @b \"B\"\n");
        check_errors(&text, &[]);
        let tree = parse(&text).unwrap().syntax();
        let campaigns = tree
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::Campaign)
            .count();
        assert_eq!(campaigns, 2);
    }

    #[test]
    fn a_role_without_a_string_is_missing_its_value() {
        check_errors(
            &in_contest("    campaign @a as\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::MissingValue,
                offset: 56,
                len: 0,
                expected: vec![Expected::Token(TokenKind::String)],
            }],
        );
    }

    #[test]
    fn a_token_after_the_identifier_is_an_unexpected_token() {
        check_errors(
            &in_contest("    campaign @a 5\n"),
            &[unexpected_token(
                54,
                1,
                vec![
                    Expected::Token(TokenKind::String),
                    Expected::Keyword(Keyword::As),
                    Expected::Token(TokenKind::Newline),
                ],
            )],
        );
    }

    #[test]
    fn a_token_after_the_name_is_an_unexpected_token() {
        check_errors(
            &in_contest("    campaign @a \"A\" 5\n"),
            &[unexpected_token(
                58,
                1,
                vec![
                    Expected::Keyword(Keyword::As),
                    Expected::Token(TokenKind::Newline),
                ],
            )],
        );
    }

    #[test]
    fn a_token_after_the_role_is_an_unexpected_token() {
        check_errors(
            &in_contest("    campaign @a as \"G\" 5\n"),
            &[unexpected_token(
                61,
                1,
                vec![Expected::Token(TokenKind::Newline)],
            )],
        );
    }

    #[test]
    fn a_flag_after_the_member_is_an_unexpected_token() {
        check_errors(
            &in_contest("    campaign @a incumbent\n"),
            &[unexpected_token(
                54,
                9,
                vec![
                    Expected::Token(TokenKind::String),
                    Expected::Keyword(Keyword::As),
                    Expected::Token(TokenKind::Newline),
                ],
            )],
        );
    }

    #[test]
    fn a_string_in_place_of_a_member_is_an_unexpected_token() {
        let text = in_contest("    campaign \"A\"\n      website https://a.example\n");
        check_errors(
            &text,
            &[unexpected_token(
                51,
                3,
                vec![
                    Expected::Keyword(Keyword::Incumbent),
                    Expected::Keyword(Keyword::Withdrawn),
                    Expected::Token(TokenKind::AtSign),
                    Expected::Token(TokenKind::Newline),
                ],
            )],
        );
        assert_eq!(
            child_nodes(&text, SyntaxKind::Campaign),
            [SyntaxKind::ErrorNode, SyntaxKind::Block]
        );
    }

    #[test]
    fn a_campaign_without_a_member_is_missing_its_block() {
        check_errors(
            &in_contest("    campaign incumbent\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::MissingBlock,
                offset: 61,
                len: 0,
                expected: vec![Expected::Token(TokenKind::Indent)],
            }],
        );
    }

    #[test]
    fn a_repeated_campaign_field_is_set_aside() {
        check_errors(
            &in_contest(
                "    campaign @a \"A\"\n      note:\n        N.\n      note:\n        M.\n",
            ),
            &[SyntaxError {
                kind: SyntaxErrorKind::RepeatedField,
                offset: 87,
                len: 17,
                expected: Vec::new(),
            }],
        );
    }

    #[test]
    fn a_contest_field_is_an_unexpected_entry_in_a_campaign() {
        check_errors(
            &in_contest("    campaign @a \"A\"\n      subtitle \"S\"\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 64,
                len: 12,
                expected: vec![
                    Expected::Keyword(Keyword::Website),
                    Expected::Keyword(Keyword::Note),
                    Expected::Keyword(Keyword::Designation),
                    Expected::Keyword(Keyword::Party),
                ],
            }],
        );
    }

    #[test]
    fn a_requirement_before_a_campaign_is_an_unexpected_entry() {
        check_errors(
            &in_contest("    required campaign @a\n"),
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 42,
                len: 20,
                expected: vec![
                    Expected::Keyword(Keyword::Subtitle),
                    Expected::Keyword(Keyword::Tabulation),
                    Expected::Keyword(Keyword::Note),
                    Expected::Keyword(Keyword::Campaign),
                ],
            }],
        );
    }
}
