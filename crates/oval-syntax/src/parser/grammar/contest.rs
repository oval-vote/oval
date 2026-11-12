use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const CONTEST_KINDS: [Expected; 4] = [
    Expected::Keyword(Keyword::Single),
    Expected::Keyword(Keyword::Multiple),
    Expected::Keyword(Keyword::Ranked),
    Expected::Keyword(Keyword::Retention),
];

const CONTEST_ENTRIES: [Expected; 4] = [
    Expected::Keyword(Keyword::Subtitle),
    Expected::Keyword(Keyword::Tabulation),
    Expected::Keyword(Keyword::Note),
    Expected::Keyword(Keyword::Campaign),
];

impl Parser<'_> {
    pub(super) fn contest(&mut self) {
        self.start_node(SyntaxKind::Contest);
        self.modifiers();
        self.bump();
        self.identifier();
        self.expect(TokenKind::String, SyntaxErrorKind::MissingName);
        self.contest_kind();
        self.end_of_line_with_block(&CONTEST_ENTRIES, Self::contest_entry);
        self.finish_node();
    }

    fn contest_kind(&mut self) {
        let Some(kind) = self.current_keyword().filter(|keyword| {
            matches!(
                keyword,
                Keyword::Single | Keyword::Multiple | Keyword::Ranked | Keyword::Retention
            )
        }) else {
            self.recover_missing(SyntaxErrorKind::MissingContestKind, CONTEST_KINDS.to_vec());
            return;
        };
        self.start_node(SyntaxKind::ContestKind);
        self.bump();
        if kind == Keyword::Multiple {
            self.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue);
        }
        let accepts = contest_modifiers(kind);
        let mut seen = Vec::new();
        while let Some(modifier) = self
            .current_keyword()
            .filter(|keyword| accepts.contains(keyword))
        {
            if seen.contains(&modifier) {
                self.set_aside(SyntaxErrorKind::RepeatedModifier, |p| {
                    p.contest_modifier(modifier);
                });
            } else {
                seen.push(modifier);
                self.contest_modifier(modifier);
            }
        }
        self.finish_node();
        if !self.at_line_end() {
            let mut expected: Vec<_> = accepts.iter().copied().map(Expected::Keyword).collect();
            expected.push(Expected::Token(TokenKind::Newline));
            self.skip_line(expected);
        }
    }

    fn contest_modifier(&mut self, modifier: Keyword) {
        let node = match modifier {
            Keyword::Seats => SyntaxKind::Seats,
            Keyword::Min => SyntaxKind::Minimum,
            Keyword::Max => SyntaxKind::RankedMax,
            _ => {
                self.bump();
                return;
            }
        };
        self.start_node(node);
        self.bump();
        self.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue);
        self.finish_node();
    }

    fn contest_entry(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let head = self.entry_head();
        let Some(keyword) = head.keyword else {
            return false;
        };
        match (keyword, head.has_modifiers()) {
            (Keyword::Subtitle | Keyword::Tabulation, false) => {
                self.field_once(seen, keyword, |p| {
                    p.value_field(SyntaxKind::StringField, TokenKind::String);
                });
            }
            (Keyword::Note, false) => self.field_once(seen, keyword, Self::markdown_field),
            (Keyword::Campaign, false) => self.campaign(),
            _ => return false,
        }
        true
    }
}

fn contest_modifiers(kind: Keyword) -> &'static [Keyword] {
    match kind {
        Keyword::Single => &[Keyword::Seats],
        Keyword::Multiple => &[Keyword::Seats, Keyword::Min],
        Keyword::Ranked => &[Keyword::Seats, Keyword::Max, Keyword::Min, Keyword::All],
        _ => &[],
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

    fn contest_kinds() -> Vec<Expected> {
        vec![
            Expected::Keyword(Keyword::Single),
            Expected::Keyword(Keyword::Multiple),
            Expected::Keyword(Keyword::Ranked),
            Expected::Keyword(Keyword::Retention),
        ]
    }

    fn missing_contest_kind(offset: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingContestKind,
            offset,
            len: 0,
            expected: contest_kinds(),
        }
    }

    fn contest_child_nodes(text: &str) -> Vec<SyntaxKind> {
        let tree = parse(text).unwrap().syntax();
        let ballot = tree.first_child().expect("the document has a ballot");
        let block = ballot.last_child().expect("the ballot has a block");
        let contest = block.first_child().expect("the block has a contest");
        contest.children().map(|node| node.kind()).collect()
    }

    #[test]
    fn a_contest_holds_its_kind_and_fields() {
        let text = "ballot @a \"A\"\n  optional extensible contest @mayor \"Mayor\" single\n    subtitle \"Vote for one\"\n    tabulation \"Plurality\"\n    note:\n      N.\n";
        check_errors(text, &[]);
        check(
            text,
            &expect![[r#"
            Document@0..140
              Ballot@0..140
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "a"
                Whitespace@9..10 " "
                String@10..13 "\"A\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..140
                  Indent@16..16 ""
                  Contest@16..140
                    Requirement@16..24
                      Word@16..24 "optional"
                    Whitespace@24..25 " "
                    Extensibility@25..35
                      Word@25..35 "extensible"
                    Whitespace@35..36 " "
                    Word@36..43 "contest"
                    Whitespace@43..44 " "
                    Identifier@44..50
                      AtSign@44..45 "@"
                      Word@45..50 "mayor"
                    Whitespace@50..51 " "
                    String@51..58 "\"Mayor\""
                    Whitespace@58..59 " "
                    ContestKind@59..65
                      Word@59..65 "single"
                    Newline@65..66 "\n"
                    Whitespace@66..70 "    "
                    Block@70..140
                      Indent@70..70 ""
                      StringField@70..94
                        Word@70..78 "subtitle"
                        Whitespace@78..79 " "
                        String@79..93 "\"Vote for one\""
                        Newline@93..94 "\n"
                      Whitespace@94..98 "    "
                      StringField@98..121
                        Word@98..108 "tabulation"
                        Whitespace@108..109 " "
                        String@109..120 "\"Plurality\""
                        Newline@120..121 "\n"
                      Whitespace@121..125 "    "
                      MarkdownField@125..140
                        Word@125..129 "note"
                        Colon@129..130 ":"
                        Newline@130..131 "\n"
                        MarkdownBlock@131..140 "      N.\n"
                      Dedent@140..140 ""
                  Dedent@140..140 ""
        "#]],
        );
    }

    #[test]
    fn each_contest_kind_is_a_node() {
        for kind in ["single", "ranked", "retention"] {
            let text = format!("ballot @a \"A\"\n  contest @m \"M\" {kind}\n    subtitle \"S\"\n");
            check_errors(&text, &[]);
            assert_eq!(
                contest_child_nodes(&text),
                [
                    SyntaxKind::Identifier,
                    SyntaxKind::ContestKind,
                    SyntaxKind::Block
                ],
                "{kind}"
            );
        }
    }

    #[test]
    fn a_contest_without_a_kind_is_missing_its_kind() {
        let text = "ballot @a \"A\"\n  contest @m \"M\"\n    subtitle \"S\"\n";
        check_errors(text, &[missing_contest_kind(30)]);
        assert_eq!(
            contest_child_nodes(text),
            [SyntaxKind::Identifier, SyntaxKind::Block]
        );
    }

    #[test]
    fn a_contest_without_a_name_is_missing_its_name_and_its_kind() {
        check_errors(
            "ballot @a \"A\"\n  contest @m\n    subtitle \"S\"\n",
            &[
                SyntaxError {
                    kind: SyntaxErrorKind::MissingName,
                    offset: 26,
                    len: 0,
                    expected: vec![Expected::Token(TokenKind::String)],
                },
                missing_contest_kind(26),
            ],
        );
    }

    #[test]
    fn a_word_that_is_not_a_contest_kind_is_an_unexpected_token() {
        let text = "ballot @a \"A\"\n  contest @m \"M\" plurality\n    subtitle \"S\"\n";
        check_errors(
            text,
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedToken,
                offset: 31,
                len: 9,
                expected: contest_kinds(),
            }],
        );
        assert_eq!(
            contest_child_nodes(text),
            [
                SyntaxKind::Identifier,
                SyntaxKind::ErrorNode,
                SyntaxKind::Block
            ]
        );
    }

    #[test]
    fn a_name_of_the_wrong_kind_does_not_also_miss_the_kind() {
        check_errors(
            "ballot @a \"A\"\n  contest @m 5\n    subtitle \"S\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedToken,
                offset: 27,
                len: 1,
                expected: vec![Expected::Token(TokenKind::String)],
            }],
        );
    }

    #[test]
    fn a_contest_without_a_block_is_missing_its_block() {
        check_errors(
            "ballot @a \"A\"\n  contest @m \"M\" single\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::MissingBlock,
                offset: 38,
                len: 0,
                expected: vec![Expected::Token(TokenKind::Indent)],
            }],
        );
    }

    #[test]
    fn a_repeated_contest_field_is_set_aside() {
        check_errors(
            "ballot @a \"A\"\n  contest @m \"M\" single\n    subtitle \"S\"\n    subtitle \"T\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::RepeatedField,
                offset: 59,
                len: 12,
                expected: Vec::new(),
            }],
        );
    }

    #[test]
    fn a_ballot_field_is_an_unexpected_entry_in_a_contest() {
        check_errors(
            "ballot @a \"A\"\n  contest @m \"M\" single\n    eligibility \"E\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 42,
                len: 15,
                expected: vec![
                    Expected::Keyword(Keyword::Subtitle),
                    Expected::Keyword(Keyword::Tabulation),
                    Expected::Keyword(Keyword::Note),
                    Expected::Keyword(Keyword::Campaign),
                ],
            }],
        );
    }

    #[test]
    fn a_section_holds_contests() {
        check_errors(
            "ballot @a \"A\"\n  required section \"S\"\n    exhaustive contest @m \"M\" single\n      subtitle \"T\"\n",
            &[],
        );
    }

    fn header(kind: &str) -> String {
        format!("ballot @a \"A\"\n  contest @m \"M\" {kind}\n    subtitle \"S\"\n")
    }

    fn modifier_nodes(text: &str) -> Vec<SyntaxKind> {
        let tree = parse(text).unwrap().syntax();
        let kind = tree
            .descendants()
            .find(|node| node.kind() == SyntaxKind::ContestKind)
            .expect("the contest has a kind");
        kind.children().map(|node| node.kind()).collect()
    }

    fn unexpected_token(offset: u32, len: u32, expected: Vec<Expected>) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedToken,
            offset,
            len,
            expected,
        }
    }

    fn missing_value(offset: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingValue,
            offset,
            len: 0,
            expected: vec![Expected::Token(TokenKind::Integer)],
        }
    }

    #[test]
    fn modifiers_are_nodes_inside_the_contest_kind() {
        let text = header("multiple 5 seats 7 min 2");
        check_errors(&text, &[]);
        check(
            &text,
            &expect![[r#"
            Document@0..73
              Ballot@0..73
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "a"
                Whitespace@9..10 " "
                String@10..13 "\"A\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..73
                  Indent@16..16 ""
                  Contest@16..73
                    Word@16..23 "contest"
                    Whitespace@23..24 " "
                    Identifier@24..26
                      AtSign@24..25 "@"
                      Word@25..26 "m"
                    Whitespace@26..27 " "
                    String@27..30 "\"M\""
                    Whitespace@30..31 " "
                    ContestKind@31..55
                      Word@31..39 "multiple"
                      Whitespace@39..40 " "
                      Integer@40..41 "5"
                      Whitespace@41..42 " "
                      Seats@42..49
                        Word@42..47 "seats"
                        Whitespace@47..48 " "
                        Integer@48..49 "7"
                      Whitespace@49..50 " "
                      Minimum@50..55
                        Word@50..53 "min"
                        Whitespace@53..54 " "
                        Integer@54..55 "2"
                    Newline@55..56 "\n"
                    Whitespace@56..60 "    "
                    Block@60..73
                      Indent@60..60 ""
                      StringField@60..73
                        Word@60..68 "subtitle"
                        Whitespace@68..69 " "
                        String@69..72 "\"S\""
                        Newline@72..73 "\n"
                      Dedent@73..73 ""
                  Dedent@73..73 ""
        "#]],
        );
    }

    #[test]
    fn each_kind_takes_its_modifiers() {
        for (kind, nodes) in [
            ("single seats 2", &[SyntaxKind::Seats][..]),
            ("multiple 5 seats 7", &[SyntaxKind::Seats]),
            (
                "multiple 3 seats 4 min 2",
                &[SyntaxKind::Seats, SyntaxKind::Minimum],
            ),
            (
                "ranked seats 3 max 6",
                &[SyntaxKind::Seats, SyntaxKind::RankedMax],
            ),
            (
                "ranked max 5 min 2",
                &[SyntaxKind::RankedMax, SyntaxKind::Minimum],
            ),
            ("retention", &[]),
        ] {
            let text = header(kind);
            check_errors(&text, &[]);
            assert_eq!(modifier_nodes(&text), nodes, "{kind}");
        }
    }

    #[test]
    fn modifiers_are_in_any_order() {
        let text = header("multiple 3 min 2 seats 4");
        check_errors(&text, &[]);
        assert_eq!(
            modifier_nodes(&text),
            [SyntaxKind::Minimum, SyntaxKind::Seats]
        );
    }

    #[test]
    fn all_is_a_word_in_the_contest_kind() {
        let text = header("ranked all");
        check_errors(&text, &[]);
        assert_eq!(modifier_nodes(&text), []);
        let tree = parse(&text).unwrap().syntax();
        let kind = tree
            .descendants()
            .find(|node| node.kind() == SyntaxKind::ContestKind)
            .expect("the contest has a kind");
        assert_eq!(kind.text().to_string(), "ranked all");
    }

    #[test]
    fn multiple_without_a_number_is_missing_its_value() {
        check_errors(&header("multiple"), &[missing_value(39)]);
    }

    #[test]
    fn a_modifier_without_a_number_is_kept_and_missing_its_value() {
        let text = header("single seats");
        check_errors(&text, &[missing_value(43)]);
        assert_eq!(modifier_nodes(&text), [SyntaxKind::Seats]);
    }

    #[test]
    fn a_modifier_the_kind_does_not_accept_is_an_unexpected_token() {
        let text = header("single max 5");
        check_errors(
            &text,
            &[unexpected_token(
                38,
                5,
                vec![
                    Expected::Keyword(Keyword::Seats),
                    Expected::Token(TokenKind::Newline),
                ],
            )],
        );
        assert_eq!(
            contest_child_nodes(&text),
            [
                SyntaxKind::Identifier,
                SyntaxKind::ContestKind,
                SyntaxKind::ErrorNode,
                SyntaxKind::Block
            ]
        );
        check_errors(
            &header("multiple 3 all"),
            &[unexpected_token(
                42,
                3,
                vec![
                    Expected::Keyword(Keyword::Seats),
                    Expected::Keyword(Keyword::Min),
                    Expected::Token(TokenKind::Newline),
                ],
            )],
        );
    }

    #[test]
    fn retention_takes_no_modifiers() {
        check_errors(
            &header("retention seats 1"),
            &[unexpected_token(
                41,
                7,
                vec![Expected::Token(TokenKind::Newline)],
            )],
        );
    }

    #[test]
    fn a_second_number_is_an_unexpected_token() {
        check_errors(
            &header("single seats 2 3"),
            &[unexpected_token(
                46,
                1,
                vec![
                    Expected::Keyword(Keyword::Seats),
                    Expected::Token(TokenKind::Newline),
                ],
            )],
        );
    }

    #[test]
    fn a_modifier_in_place_of_the_number_of_votes_is_an_unexpected_token() {
        check_errors(
            &header("multiple seats 7"),
            &[unexpected_token(
                40,
                7,
                vec![Expected::Token(TokenKind::Integer)],
            )],
        );
    }

    #[test]
    fn a_modifier_with_a_string_is_an_unexpected_token() {
        check_errors(
            &header("single seats \"3\""),
            &[unexpected_token(
                44,
                3,
                vec![Expected::Token(TokenKind::Integer)],
            )],
        );
    }

    fn repeated_modifier(offset: u32, len: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::RepeatedModifier,
            offset,
            len,
            expected: Vec::new(),
        }
    }

    #[test]
    fn a_repeated_modifier_is_set_aside_with_its_number() {
        let text = header("ranked max 6 max 4");
        check_errors(&text, &[repeated_modifier(44, 5)]);
        check(
            &text,
            &expect![[r#"
            Document@0..67
              Ballot@0..67
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "a"
                Whitespace@9..10 " "
                String@10..13 "\"A\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..67
                  Indent@16..16 ""
                  Contest@16..67
                    Word@16..23 "contest"
                    Whitespace@23..24 " "
                    Identifier@24..26
                      AtSign@24..25 "@"
                      Word@25..26 "m"
                    Whitespace@26..27 " "
                    String@27..30 "\"M\""
                    Whitespace@30..31 " "
                    ContestKind@31..49
                      Word@31..37 "ranked"
                      Whitespace@37..38 " "
                      RankedMax@38..43
                        Word@38..41 "max"
                        Whitespace@41..42 " "
                        Integer@42..43 "6"
                      Whitespace@43..44 " "
                      ErrorNode@44..49
                        RankedMax@44..49
                          Word@44..47 "max"
                          Whitespace@47..48 " "
                          Integer@48..49 "4"
                    Newline@49..50 "\n"
                    Whitespace@50..54 "    "
                    Block@54..67
                      Indent@54..54 ""
                      StringField@54..67
                        Word@54..62 "subtitle"
                        Whitespace@62..63 " "
                        String@63..66 "\"S\""
                        Newline@66..67 "\n"
                      Dedent@67..67 ""
                  Dedent@67..67 ""
        "#]],
        );
    }

    #[test]
    fn a_repeated_all_is_set_aside() {
        check_errors(&header("ranked all all"), &[repeated_modifier(42, 3)]);
    }

    #[test]
    fn each_repetition_is_reported() {
        check_errors(
            &header("ranked all all all"),
            &[repeated_modifier(42, 3), repeated_modifier(46, 3)],
        );
    }

    #[test]
    fn modifiers_after_a_repeated_one_are_still_parsed() {
        let text = header("ranked max 6 max 4 min 2");
        check_errors(&text, &[repeated_modifier(44, 5)]);
        assert_eq!(
            modifier_nodes(&text),
            [
                SyntaxKind::RankedMax,
                SyntaxKind::ErrorNode,
                SyntaxKind::Minimum
            ]
        );
    }

    #[test]
    fn errors_inside_a_repeated_modifier_follow_it() {
        check_errors(
            &header("single seats 2 seats"),
            &[repeated_modifier(46, 5), missing_value(51)],
        );
    }

    #[test]
    fn a_modifier_the_kind_does_not_accept_is_not_also_repeated() {
        check_errors(
            &header("single max 5 max 6"),
            &[unexpected_token(
                38,
                11,
                vec![
                    Expected::Keyword(Keyword::Seats),
                    Expected::Token(TokenKind::Newline),
                ],
            )],
        );
    }
}
