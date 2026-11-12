use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

const SECTION_ENTRIES: [Expected; 1] = [Expected::Keyword(Keyword::Note)];

impl Parser<'_> {
    pub(super) fn section(&mut self) {
        self.start_node(SyntaxKind::Section);
        self.modifiers();
        self.bump();
        self.expect(TokenKind::String, SyntaxErrorKind::MissingName);
        self.end_of_line_with_block(&SECTION_ENTRIES, Self::section_entry);
        self.finish_node();
    }

    fn section_entry(&mut self, seen: &mut Vec<Keyword>) -> bool {
        let head = self.entry_head();
        let Some(keyword) = head.keyword else {
            return false;
        };
        match (keyword, head.has_modifiers()) {
            (Keyword::Note, false) => self.field_once(seen, keyword, Self::markdown_field),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::tests::check;
    use crate::{Expected, Keyword, SyntaxError, SyntaxErrorKind, TokenKind, parse};
    use expect_test::expect;

    fn check_errors(text: &str, expected: &[SyntaxError]) {
        assert_eq!(parse(text).unwrap().syntax_errors, expected, "{text:?}");
    }

    fn unexpected_entry(offset: u32, len: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedEntry,
            offset,
            len,
            expected: vec![Expected::Keyword(Keyword::Note)],
        }
    }

    #[test]
    fn a_section_holds_its_note() {
        let text = "ballot @a \"A\"\n  required exhaustive section \"City Offices\"\n    note:\n      Local races.\n";
        check_errors(text, &[]);
        check(
            text,
            &expect![[r#"
            Document@0..88
              Ballot@0..88
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "a"
                Whitespace@9..10 " "
                String@10..13 "\"A\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..88
                  Indent@16..16 ""
                  Section@16..88
                    Requirement@16..24
                      Word@16..24 "required"
                    Whitespace@24..25 " "
                    Extensibility@25..35
                      Word@25..35 "exhaustive"
                    Whitespace@35..36 " "
                    Word@36..43 "section"
                    Whitespace@43..44 " "
                    String@44..58 "\"City Offices\""
                    Newline@58..59 "\n"
                    Whitespace@59..63 "    "
                    Block@63..88
                      Indent@63..63 ""
                      MarkdownField@63..88
                        Word@63..67 "note"
                        Colon@67..68 ":"
                        Newline@68..69 "\n"
                        MarkdownBlock@69..88 "      Local races.\n"
                      Dedent@88..88 ""
                  Dedent@88..88 ""
        "#]],
        );
    }

    #[test]
    fn a_section_without_a_title_is_missing_a_name() {
        check_errors(
            "ballot @a \"A\"\n  section\n    note:\n      N.\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::MissingName,
                offset: 23,
                len: 0,
                expected: vec![Expected::Token(TokenKind::String)],
            }],
        );
    }

    #[test]
    fn a_section_without_a_block_is_missing_its_block() {
        check_errors(
            "ballot @a \"A\"\n  section \"S\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::MissingBlock,
                offset: 28,
                len: 0,
                expected: vec![Expected::Token(TokenKind::Indent)],
            }],
        );
    }

    #[test]
    fn a_section_has_no_identifier() {
        check_errors(
            "ballot @a \"A\"\n  section @x \"X\"\n    note:\n      N.\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedToken,
                offset: 24,
                len: 6,
                expected: vec![Expected::Token(TokenKind::String)],
            }],
        );
    }

    #[test]
    fn sections_do_not_nest() {
        check_errors(
            "ballot @a \"A\"\n  section \"S\"\n    section \"T\"\n",
            &[unexpected_entry(32, 11)],
        );
    }

    #[test]
    fn a_section_does_not_hold_sub_ballots() {
        check_errors(
            "ballot @a \"A\"\n  section \"S\"\n    ballot @b \"B\"\n",
            &[unexpected_entry(32, 13)],
        );
    }

    #[test]
    fn a_modified_section_inside_a_section_is_an_unexpected_entry() {
        check_errors(
            "ballot @a \"A\"\n  section \"S\"\n    required section \"T\"\n",
            &[unexpected_entry(32, 20)],
        );
    }

    #[test]
    fn a_section_is_not_a_document_entry() {
        let errors = parse("section \"S\"\n").unwrap().syntax_errors;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, SyntaxErrorKind::UnexpectedEntry);
        assert_eq!((errors[0].offset, errors[0].len), (0, 11));
    }
}
