use super::Parser;
use crate::SyntaxKind::{
    self, Ballot, BallotKeyword, ContestKeyword, EligibilityKeyword, MeasureKeyword, NoteKeyword,
    Section, SectionKeyword, StringLiteral,
};

pub(super) const BALLOT_FIELDS: &[SyntaxKind] = &[EligibilityKeyword, NoteKeyword];
pub(super) const BALLOT_ITEMS: &[SyntaxKind] = &[
    SectionKeyword,
    ContestKeyword,
    MeasureKeyword,
    BallotKeyword,
];
pub(super) const SECTION_FIELDS: &[SyntaxKind] = &[NoteKeyword];
pub(super) const SECTION_ITEMS: &[SyntaxKind] = &[ContestKeyword, MeasureKeyword];

impl Parser<'_> {
    pub(super) fn ballot(&mut self) {
        self.start_node(Ballot);
        self.bump();
        self.declaration_header();
        self.required_block(BALLOT_FIELDS, BALLOT_ITEMS, Self::ballot_item);
        self.finish_node();
    }

    pub(super) fn ballot_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            SectionKeyword => self.section(),
            BallotKeyword => self.ballot(),
            ContestKeyword => self.contest(),
            MeasureKeyword => self.measure(),
            EligibilityKeyword | NoteKeyword => return self.simple_field(kind),
            _ => return false,
        }
        true
    }

    pub(super) fn section(&mut self) {
        self.start_node(Section);
        self.bump();
        self.expect_value(StringLiteral);
        self.end_of_line();
        self.required_block(SECTION_FIELDS, SECTION_ITEMS, Self::section_item);
        self.finish_node();
    }

    pub(super) fn section_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            ContestKeyword => self.contest(),
            MeasureKeyword => self.measure(),
            NoteKeyword => return self.simple_field(kind),
            _ => return false,
        }
        true
    }
}
