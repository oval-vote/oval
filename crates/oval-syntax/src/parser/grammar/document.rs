use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxKind};

const DOCUMENT_ENTRIES: [Expected; 8] = [
    Expected::Keyword(Keyword::Oval),
    Expected::Keyword(Keyword::Guide),
    Expected::Keyword(Keyword::Election),
    Expected::Keyword(Keyword::Date),
    Expected::Keyword(Keyword::Note),
    Expected::Keyword(Keyword::Party),
    Expected::Keyword(Keyword::Candidate),
    Expected::Keyword(Keyword::Ballot),
];

impl Parser<'_> {
    pub(in crate::parser) fn document(&mut self) {
        self.start_node_here(SyntaxKind::Document);
        self.entries(&DOCUMENT_ENTRIES, Self::document_entry);
        self.eat_skipped();
        self.finish_node();
    }

    fn document_entry(&mut self, seen: &mut Vec<Keyword>) -> bool {
        if self.at_keyword(Keyword::Oval) {
            self.version_marker();
        } else if self.entry_head().keyword == Some(Keyword::Ballot) {
            self.ballot();
        } else if !self.guide_field(seen) && !self.declaration() {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::tests::check;
    use crate::{Expected, Keyword, SyntaxError, SyntaxErrorKind, SyntaxKind, parse};
    use expect_test::expect;

    fn check_errors(text: &str, expected: &[SyntaxError]) {
        assert_eq!(parse(text).unwrap().syntax_errors, expected, "{text:?}");
    }

    fn unexpected_entry(offset: u32, len: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedEntry,
            offset,
            len,
            expected: vec![
                Expected::Keyword(Keyword::Oval),
                Expected::Keyword(Keyword::Guide),
                Expected::Keyword(Keyword::Election),
                Expected::Keyword(Keyword::Date),
                Expected::Keyword(Keyword::Note),
                Expected::Keyword(Keyword::Party),
                Expected::Keyword(Keyword::Candidate),
                Expected::Keyword(Keyword::Ballot),
            ],
        }
    }

    #[test]
    fn a_document_holds_every_kind_of_entry() {
        let text = "oval 1\nguide \"G\"\nparty @d \"D\"\ncandidate @c \"C\"\nballot @b \"B\"\n  eligibility \"E\"\n";
        let result = parse(text).unwrap();
        assert_eq!(result.syntax_errors, []);
        let kinds: Vec<_> = result.syntax().children().map(|node| node.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::VersionMarker,
                SyntaxKind::StringField,
                SyntaxKind::Party,
                SyntaxKind::Candidate,
                SyntaxKind::Ballot,
            ]
        );
    }

    #[test]
    fn a_line_no_document_entry_begins_with_is_an_unexpected_entry() {
        let text = "abbreviation \"D\"\nguide \"G\"\n";
        check_errors(text, &[unexpected_entry(0, 16)]);
        check(
            text,
            &expect![[r#"
            Document@0..27
              ErrorNode@0..17
                Word@0..12 "abbreviation"
                Whitespace@12..13 " "
                String@13..16 "\"D\""
                Newline@16..17 "\n"
              StringField@17..27
                Word@17..22 "guide"
                Whitespace@22..23 " "
                String@23..26 "\"G\""
                Newline@26..27 "\n"
        "#]],
        );
    }

    #[test]
    fn a_string_cannot_begin_a_document_entry() {
        check_errors("\"Guide\"\n", &[unexpected_entry(0, 7)]);
    }

    #[test]
    fn an_unexpected_entry_at_the_top_is_skipped_with_its_block() {
        check_errors(
            "contest @mayor \"Mayor\" single\n  campaign @a \"A\"\nguide \"G\"\n",
            &[unexpected_entry(0, 47)],
        );
    }

    #[test]
    fn a_block_below_a_version_marker_is_unexpected() {
        check_errors(
            "oval 1\n  guide \"G\"\n",
            &[SyntaxError {
                kind: SyntaxErrorKind::UnexpectedBlock,
                offset: 9,
                len: 9,
                expected: Vec::new(),
            }],
        );
    }

    #[test]
    fn a_modifier_before_a_declaration_is_an_unexpected_entry() {
        check_errors("required party @d \"D\"\n", &[unexpected_entry(0, 21)]);
    }

    #[test]
    fn a_modifier_alone_does_not_take_the_next_line() {
        check_errors(
            "required\nballot @b \"B\"\n  eligibility \"E\"\n",
            &[unexpected_entry(0, 8)],
        );
    }
}
