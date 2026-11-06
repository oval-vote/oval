use super::Parser;
use crate::SyntaxKind::{
    self, AbbreviationKeyword, BallotKeyword, Candidate, CandidateKeyword, DateKeyword,
    DesignationKeyword, Document, ElectionKeyword, GuideKeyword, Identifier, IntegerLiteral,
    NoteKeyword, OvalKeyword, Party, PartyKeyword, PartyReference, StringLiteral, VersionMarker,
    WebsiteKeyword,
};

pub(super) const GUIDE_FIELDS: &[SyntaxKind] =
    &[GuideKeyword, ElectionKeyword, DateKeyword, NoteKeyword];
pub(super) const TOP_LEVEL: &[SyntaxKind] = &[
    OvalKeyword,
    GuideKeyword,
    ElectionKeyword,
    DateKeyword,
    NoteKeyword,
    PartyKeyword,
    CandidateKeyword,
    BallotKeyword,
];
pub(super) const PARTY_FIELDS: &[SyntaxKind] = &[AbbreviationKeyword, WebsiteKeyword, NoteKeyword];
pub(super) const CANDIDATE_FIELDS: &[SyntaxKind] = &[DesignationKeyword, PartyKeyword, NoteKeyword];

impl Parser<'_> {
    pub(super) fn document(&mut self) {
        self.builder.start_node(Document.into());
        let mut remaining = TOP_LEVEL.to_vec();
        while let Some(kind) = self.current() {
            match kind {
                OvalKeyword => {
                    self.field_once(kind, &mut remaining, Self::version_marker);
                }
                GuideKeyword | ElectionKeyword | DateKeyword | NoteKeyword => {
                    remaining.retain(|k| *k != OvalKeyword);
                    self.field_once(kind, &mut remaining, Self::simple_field);
                }
                PartyKeyword | CandidateKeyword => {
                    if remaining.contains(&kind) {
                        remaining.retain(|k| *k != OvalKeyword && !GUIDE_FIELDS.contains(k));
                        self.declaration(kind);
                    } else {
                        self.misplaced(kind, &remaining, Self::declaration);
                    }
                }
                BallotKeyword => {
                    remaining.retain(|k| *k == BallotKeyword);
                    self.ballot();
                }
                _ => self.recover_item(&remaining),
            }
        }
        self.eat_trivia();
        self.finish_node();
    }

    pub(super) fn version_marker(&mut self, _kind: SyntaxKind) -> bool {
        self.start_node(VersionMarker);
        self.bump();
        self.expect_value(IntegerLiteral);
        self.end_of_line();
        self.finish_node();
        true
    }

    pub(super) fn declaration(&mut self, kind: SyntaxKind) -> bool {
        if kind == PartyKeyword {
            self.party();
        } else {
            self.candidate();
        }
        true
    }

    pub(super) fn party(&mut self) {
        self.start_node(Party);
        self.bump();
        self.declaration_header();
        self.block(PARTY_FIELDS, &[], Self::simple_field);
        self.finish_node();
    }

    pub(super) fn candidate(&mut self) {
        self.start_node(Candidate);
        self.bump();
        self.declaration_header();
        self.block(CANDIDATE_FIELDS, &[], Self::candidate_field);
        self.finish_node();
    }

    pub(super) fn candidate_field(&mut self, kind: SyntaxKind) -> bool {
        if kind == PartyKeyword {
            self.party_reference();
            true
        } else {
            self.simple_field(kind)
        }
    }

    pub(super) fn party_reference(&mut self) {
        self.start_node(PartyReference);
        self.bump();
        self.expect(Identifier);
        if self.eat(StringLiteral) {
            self.end_of_line();
            self.block(PARTY_FIELDS, &[], Self::simple_field);
        } else {
            self.end_of_line();
        }
        self.finish_node();
    }
}
