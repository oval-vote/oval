use std::collections::HashSet;

use oval_syntax::SyntaxNode;
use oval_syntax::ast::{AstNode, AstToken, Ballot, BallotItem, Document, Section};

use super::{diagnostic, header_range, kept_nodes};
use crate::{Diagnostic, ValidationRule};

pub(super) fn empty_ballots(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for ballot in kept_nodes(document.syntax()).filter_map(Ballot::cast) {
        let has_fields = ballot.eligibility().is_some() || ballot.note().is_some();
        if has_fields && ballot.items().next().is_none() {
            push_header(ValidationRule::EmptyBallot, ballot.syntax(), diagnostics);
        }
    }
}

pub(super) fn empty_sections(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for section in kept_nodes(document.syntax()).filter_map(Section::cast) {
        if section.note().is_some() && section.items().next().is_none() {
            push_header(ValidationRule::EmptySection, section.syntax(), diagnostics);
        }
    }
}

pub(super) fn duplicate_ballot_titles(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    duplicate_titles(document.ballots(), diagnostics);
    for ballot in kept_nodes(document.syntax()).filter_map(Ballot::cast) {
        let sub_ballots = ballot.items().filter_map(|item| match item {
            BallotItem::Ballot(sub_ballot) => Some(sub_ballot),
            _ => None,
        });
        duplicate_titles(sub_ballots, diagnostics);
    }
}

fn duplicate_titles(siblings: impl Iterator<Item = Ballot>, diagnostics: &mut Vec<Diagnostic>) {
    let mut titles = HashSet::new();
    for title in siblings.filter_map(|ballot| ballot.name()) {
        if !titles.insert(title.value().into_owned()) {
            diagnostics.push(diagnostic(
                ValidationRule::DuplicateBallotTitle,
                title.syntax().text_range(),
            ));
        }
    }
}

fn push_header(rule: ValidationRule, node: &SyntaxNode, diagnostics: &mut Vec<Diagnostic>) {
    let range = header_range(node).expect("a ballot or section begins with its keyword");
    diagnostics.push(diagnostic(rule, range));
}

#[cfg(test)]
mod tests {
    use crate::validate::{reported, reported_as};

    #[test]
    fn a_ballot_or_section_with_fields_but_no_items_is_empty() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  eligibility \"E\"\nrequired ballot @e \"E\"  # c\n  note:\n    N.\nballot @c \"C\"\n  section \"S\"\n    note:\n      N.\n  ballot @d \"D\"\n    bogus 1\n  measure @m \"M\"\n    recommend yes\n";
        assert_eq!(reported_as(text, "empty-ballot"), [(46, 59), (78, 100)]);
        assert_eq!(reported_as(text, "empty-section"), [(137, 148)]);
    }

    #[test]
    fn a_ballot_whose_only_entries_are_discarded_has_no_fields() {
        let text =
            "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  bogus 1\n";
        assert_eq!(reported(text), [("unexpected-entry", 62, 69)]);
    }

    #[test]
    fn root_ballots_or_sub_ballots_of_one_ballot_with_the_same_title_are_duplicates() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @a \"City\"\n  ballot @x \"Sub\"\n    measure @m1 \"M\"\n      recommend yes\n  ballot @y \"Sub\"\n    measure @m2 \"M\"\n      recommend yes\nballot @b \"City\"\n  ballot @z \"Sub\"\n    measure @m3 \"M\"\n      recommend yes\n";
        assert_eq!(
            reported(text),
            [
                ("duplicate-ballot-title", 133, 138),
                ("duplicate-ballot-title", 189, 195)
            ]
        );
    }
}
