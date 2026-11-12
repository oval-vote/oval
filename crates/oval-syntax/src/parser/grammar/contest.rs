use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const CONTEST_KINDS: [Expected; 4] = [
    Expected::Keyword(Keyword::Single),
    Expected::Keyword(Keyword::Multiple),
    Expected::Keyword(Keyword::Ranked),
    Expected::Keyword(Keyword::Retention),
];

const CONTEST_ENTRIES: [Expected; 3] = [
    Expected::Keyword(Keyword::Subtitle),
    Expected::Keyword(Keyword::Tabulation),
    Expected::Keyword(Keyword::Note),
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
        if matches!(
            self.current_keyword(),
            Some(Keyword::Single | Keyword::Multiple | Keyword::Ranked | Keyword::Retention)
        ) {
            self.start_node(SyntaxKind::ContestKind);
            self.bump();
            self.finish_node();
        } else {
            self.recover_missing(SyntaxErrorKind::MissingContestKind, CONTEST_KINDS.to_vec());
        }
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
}
