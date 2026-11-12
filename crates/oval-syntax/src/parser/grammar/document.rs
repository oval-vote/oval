use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind};

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    VersionMarker,
    GuideFields,
    Declarations,
    Ballots,
}

#[derive(Default)]
struct Order {
    started: bool,
    latest: Option<Group>,
}

impl Order {
    fn is_misplaced(&self, group: Group) -> bool {
        if group == Group::VersionMarker {
            self.started
        } else {
            self.latest > Some(group)
        }
    }
}

impl Parser<'_> {
    pub(in crate::parser) fn document(&mut self) {
        self.start_node_here(SyntaxKind::Document);
        let mut order = Order::default();
        self.entries(&DOCUMENT_ENTRIES, |p, seen| {
            p.document_entry(seen, &mut order)
        });
        self.eat_skipped();
        self.finish_node();
    }

    fn document_entry(&mut self, seen: &mut Vec<Keyword>, order: &mut Order) -> bool {
        let Some(group) = self.document_group() else {
            order.started = true;
            return false;
        };
        if order.is_misplaced(group) {
            self.set_aside(SyntaxErrorKind::MisplacedEntry, |p| {
                p.group_entry(group, &mut Vec::new());
            });
        } else {
            order.latest = Some(group);
            self.group_entry(group, seen);
        }
        order.started = true;
        true
    }

    fn document_group(&self) -> Option<Group> {
        let head = self.entry_head();
        let keyword = head.keyword?;
        if keyword == Keyword::Ballot {
            return Some(Group::Ballots);
        }
        if head.has_modifiers() {
            return None;
        }
        match keyword {
            Keyword::Oval => Some(Group::VersionMarker),
            Keyword::Guide | Keyword::Election | Keyword::Date | Keyword::Note => {
                Some(Group::GuideFields)
            }
            Keyword::Party | Keyword::Candidate => Some(Group::Declarations),
            _ => None,
        }
    }

    fn group_entry(&mut self, group: Group, seen: &mut Vec<Keyword>) {
        match group {
            Group::VersionMarker => self.version_marker(),
            Group::GuideFields => {
                self.guide_field(seen);
            }
            Group::Declarations => {
                self.declaration();
            }
            Group::Ballots => self.ballot(),
        }
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

    fn misplaced(offset: u32, len: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MisplacedEntry,
            offset,
            len,
            expected: Vec::new(),
        }
    }

    #[test]
    fn a_guide_field_after_the_first_ballot_is_set_aside() {
        let text = "ballot @b \"B\"\n  eligibility \"E\"\ndate 2026-11-03\n";
        check_errors(text, &[misplaced(32, 15)]);
        check(
            text,
            &expect![[r#"
            Document@0..48
              Ballot@0..32
                Word@0..6 "ballot"
                Whitespace@6..7 " "
                Identifier@7..9
                  AtSign@7..8 "@"
                  Word@8..9 "b"
                Whitespace@9..10 " "
                String@10..13 "\"B\""
                Newline@13..14 "\n"
                Whitespace@14..16 "  "
                Block@16..32
                  Indent@16..16 ""
                  StringField@16..32
                    Word@16..27 "eligibility"
                    Whitespace@27..28 " "
                    String@28..31 "\"E\""
                    Newline@31..32 "\n"
                  Dedent@32..32 ""
              ErrorNode@32..48
                DateField@32..48
                  Word@32..36 "date"
                  Whitespace@36..37 " "
                  Date@37..47 "2026-11-03"
                  Newline@47..48 "\n"
        "#]],
        );
    }

    #[test]
    fn a_guide_field_after_a_declaration_is_misplaced() {
        check_errors("party @d \"D\"\nguide \"G\"\n", &[misplaced(13, 9)]);
    }

    #[test]
    fn a_declaration_after_the_first_ballot_is_set_aside_with_its_block() {
        check_errors(
            "ballot @b \"B\"\n  eligibility \"E\"\nparty @d \"D\"\n  abbreviation \"D\"\n",
            &[misplaced(32, 31)],
        );
    }

    #[test]
    fn a_version_marker_after_another_entry_is_misplaced() {
        check_errors("guide \"G\"\noval 1\n", &[misplaced(10, 6)]);
        check_errors("oval 1\noval 1\n", &[misplaced(7, 6)]);
    }

    #[test]
    fn a_version_marker_after_an_unexpected_entry_is_misplaced() {
        check_errors("junk\noval 1\n", &[unexpected_entry(0, 4), misplaced(5, 6)]);
    }

    #[test]
    fn entries_within_a_group_are_in_any_order() {
        check_errors(
            "oval 1\ndate 2026-11-03\nguide \"G\"\ncandidate @c \"C\"\nparty @d \"D\"\nballot @a \"A\"\n  eligibility \"E\"\nballot @b \"B\"\n  eligibility \"E\"\n",
            &[],
        );
    }

    #[test]
    fn a_misplaced_guide_field_is_not_also_repeated() {
        check_errors(
            "date 2026-11-03\nparty @d \"D\"\ndate 2026-11-04\n",
            &[misplaced(29, 15)],
        );
    }

    #[test]
    fn errors_inside_a_misplaced_entry_follow_it() {
        check_errors(
            "party @d \"D\"\nguide 5\n",
            &[
                misplaced(13, 7),
                SyntaxError {
                    kind: SyntaxErrorKind::UnexpectedToken,
                    offset: 19,
                    len: 1,
                    expected: vec![Expected::Token(TokenKind::String)],
                },
            ],
        );
    }

    #[test]
    fn a_modifier_alone_does_not_take_the_next_line() {
        check_errors(
            "required\nballot @b \"B\"\n  eligibility \"E\"\n",
            &[unexpected_entry(0, 8)],
        );
    }
}
