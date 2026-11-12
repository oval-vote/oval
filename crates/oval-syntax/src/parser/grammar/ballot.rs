use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const BALLOT_ENTRIES: [Expected; 3] = [
    Expected::Keyword(Keyword::Eligibility),
    Expected::Keyword(Keyword::Note),
    Expected::Keyword(Keyword::Ballot),
];

impl Parser<'_> {
    pub(super) fn ballot(&mut self) {
        self.start_node(SyntaxKind::Ballot);
        self.bump();
        self.identifier();
        self.expect(TokenKind::String, SyntaxErrorKind::MissingName);
        self.end_of_line_with_block(&BALLOT_ENTRIES, Self::ballot_entry);
        self.finish_node();
    }

    fn ballot_entry(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };
        match keyword {
            Keyword::Eligibility => self.field_once(seen, keyword, |p| {
                p.value_field(SyntaxKind::StringField, TokenKind::String);
            }),
            Keyword::Note => self.field_once(seen, keyword, Self::markdown_field),
            Keyword::Ballot => self.ballot(),
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

    fn missing_block(offset: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingBlock,
            offset,
            len: 0,
            expected: vec![Expected::Token(TokenKind::Indent)],
        }
    }

    fn document_kinds(text: &str) -> Vec<SyntaxKind> {
        parse(text)
            .unwrap()
            .syntax()
            .children()
            .map(|node| node.kind())
            .collect()
    }

    #[test]
    fn a_ballot_holds_its_fields_and_sub_ballots() {
        let text = "ballot @franklin \"State of Franklin\"\n  eligibility \"Registered voters\"\n  ballot @ashford \"City of Ashford\"\n    note:\n      City races.\n";
        check_errors(text, &[]);
        check(
            text,
            &expect![[r#"
            Document@0..135
              Ballot@0..135
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..16
                  AtSign@7..8 "@"
                  Word@8..16 "franklin"
                Whitespace@16..17 " "
                String@17..36 "\"State of Franklin\""
                Newline@36..37 "\n"
                Whitespace@37..39 "  "
                Block@39..135
                  Indent@39..39 ""
                  StringField@39..71
                    Word@39..50 "eligibility"
                    Whitespace@50..51 " "
                    String@51..70 "\"Registered voters\""
                    Newline@70..71 "\n"
                  Whitespace@71..73 "  "
                  Ballot@73..135
                    Word@73..79 "ballot"
                    Whitespace@79..80 " "
                    Identifier@80..88
                      AtSign@80..81 "@"
                      Word@81..88 "ashford"
                    Whitespace@88..89 " "
                    String@89..106 "\"City of Ashford\""
                    Newline@106..107 "\n"
                    Whitespace@107..111 "    "
                    Block@111..135
                      Indent@111..111 ""
                      MarkdownField@111..135
                        Word@111..115 "note"
                        Colon@115..116 ":"
                        Newline@116..117 "\n"
                        MarkdownBlock@117..135 "      City races.\n"
                      Dedent@135..135 ""
                  Dedent@135..135 ""
        "#]],
        );
    }

    #[test]
    fn ballots_follow_the_declarations() {
        let text = "party @dem \"D\"\nballot @a \"A\"\n  eligibility \"E\"\nballot @b \"B\"\n  eligibility \"E\"\n";
        check_errors(text, &[]);
        assert_eq!(
            document_kinds(text),
            [SyntaxKind::Party, SyntaxKind::Ballot, SyntaxKind::Ballot]
        );
    }

    #[test]
    fn a_ballot_without_a_block_is_missing_its_block() {
        check_errors("ballot @city \"City\"\n", &[missing_block(20)]);
    }

    #[test]
    fn a_ballot_missing_its_block_is_kept() {
        let text = "ballot @a \"A\"\nballot @b \"B\"\n  eligibility \"E\"\n";
        check_errors(text, &[missing_block(14)]);
        assert_eq!(
            document_kinds(text),
            [SyntaxKind::Ballot, SyntaxKind::Ballot]
        );
    }

    #[test]
    fn a_ballot_can_be_missing_its_block_at_the_end_of_input() {
        check_errors("ballot @a \"A\"", &[missing_block(13)]);
    }

    #[test]
    fn a_sub_ballot_can_be_missing_its_block() {
        check_errors("ballot @a \"A\"\n  ballot @b \"B\"\n", &[missing_block(30)]);
    }

    #[test]
    fn a_missing_block_is_not_reported_on_a_line_with_an_error_token() {
        let result = parse("ballot @a \"A\" ~\n").unwrap();
        assert_eq!(result.lexical_errors.len(), 1);
        assert_eq!(result.syntax_errors, []);
    }

    #[test]
    fn a_ballot_with_a_skipped_line_is_still_missing_its_block() {
        check_errors(
            "ballot @a 5\n",
            &[
                SyntaxError {
                    kind: SyntaxErrorKind::UnexpectedToken,
                    offset: 10,
                    len: 1,
                    expected: vec![Expected::Token(TokenKind::String)],
                },
                missing_block(12),
            ],
        );
    }

    #[test]
    fn a_candidate_field_is_an_unexpected_entry_in_a_ballot() {
        check_errors(
            "ballot @a \"A\"\n  designation \"D\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 16,
                len: 15,
                expected: vec![
                    Expected::Keyword(Keyword::Eligibility),
                    Expected::Keyword(Keyword::Note),
                    Expected::Keyword(Keyword::Ballot),
                ],
            }],
        );
    }
}
