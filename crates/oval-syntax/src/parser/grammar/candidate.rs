use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const CANDIDATE_FIELDS: [Expected; 2] = [
    Expected::Keyword(Keyword::Designation),
    Expected::Keyword(Keyword::Note),
];

impl Parser<'_> {
    pub(super) fn candidate(&mut self) {
        self.start_node(SyntaxKind::Candidate);
        self.bump();
        self.identifier();
        self.expect(TokenKind::String, SyntaxErrorKind::MissingName);
        self.end_of_line();
        if self.at(TokenKind::Indent) {
            self.block(&CANDIDATE_FIELDS, Self::candidate_field);
        }
        self.finish_node();
    }

    fn candidate_field(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };
        match keyword {
            Keyword::Designation => self.field_once(seen, keyword, |p| {
                p.value_field(SyntaxKind::StringField, TokenKind::String);
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
    use crate::{Expected, Keyword, SyntaxError, SyntaxErrorKind, SyntaxKind, TokenKind, parse};
    use expect_test::expect;

    fn check_errors(text: &str, expected: &[SyntaxError]) {
        assert_eq!(parse(text).unwrap().syntax_errors, expected, "{text:?}");
    }

    #[test]
    fn a_candidate_block_holds_its_fields() {
        let text = "candidate @nreyes \"Nadia Reyes\"\n  designation \"United States Senator\"\n  note:\n    Appointed in January 2025.\n";
        check_errors(text, &[]);
        check(
            text,
            &expect![[r#"
            Document@0..109
              Candidate@0..109
                Word@0..9 "candidate"
                Whitespace@9..10 " "
                Identifier@10..17
                  AtSign@10..11 "@"
                  Word@11..17 "nreyes"
                Whitespace@17..18 " "
                String@18..31 "\"Nadia Reyes\""
                Newline@31..32 "\n"
                Whitespace@32..34 "  "
                Block@34..109
                  Indent@34..34 ""
                  StringField@34..70
                    Word@34..45 "designation"
                    Whitespace@45..46 " "
                    String@46..69 "\"United States Senator\""
                    Newline@69..70 "\n"
                  Whitespace@70..72 "  "
                  MarkdownField@72..109
                    Word@72..76 "note"
                    Colon@76..77 ":"
                    Newline@77..78 "\n"
                    MarkdownBlock@78..109 "    Appointed in January 2025.\n"
                  Dedent@109..109 ""
        "#]],
        );
    }

    #[test]
    fn candidates_and_parties_are_declared_in_any_order() {
        let text = "party @dem \"D\"\ncandidate @nreyes \"Nadia Reyes\"\nparty @rep \"R\"\n";
        let result = parse(text).unwrap();
        assert_eq!(result.syntax_errors, []);
        let kinds: Vec<_> = result.syntax().children().map(|node| node.kind()).collect();
        assert_eq!(
            kinds,
            [SyntaxKind::Party, SyntaxKind::Candidate, SyntaxKind::Party]
        );
    }

    #[test]
    fn a_candidate_without_a_name_is_missing_a_name() {
        check_errors(
            "candidate @nreyes\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::MissingName,
                offset: 17,
                len: 0,
                expected: vec![Expected::Token(TokenKind::String)],
            }],
        );
    }

    #[test]
    fn a_repeated_field_in_a_candidate_block_is_set_aside() {
        check_errors(
            "candidate @c \"C\"\n  designation \"A\"\n  designation \"B\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::RepeatedField,
                offset: 37,
                len: 15,
                expected: Vec::new(),
            }],
        );
    }

    #[test]
    fn a_party_field_is_an_unexpected_entry_in_a_candidate_block() {
        check_errors(
            "candidate @c \"C\"\n  website https://x.example\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedEntry,
                offset: 19,
                len: 25,
                expected: vec![
                    Expected::Keyword(Keyword::Designation),
                    Expected::Keyword(Keyword::Note),
                ],
            }],
        );
    }
}
