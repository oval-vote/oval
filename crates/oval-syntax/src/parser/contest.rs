use super::Parser;
use super::document::CANDIDATE_FIELDS;
use crate::SyntaxKind::{
    self, AsKeyword, At, Campaign, CampaignKeyword, Contest, ContestKind, DesignationKeyword, Flag,
    IncumbentKeyword, IntegerLiteral, MaxKeyword, Member, MultipleKeyword, Newline, NoteKeyword,
    PartyKeyword, RankedKeyword, RecommendKeyword, RetentionKeyword, SingleKeyword, StringLiteral,
    SubtitleKeyword, TabulationKeyword, WebsiteKeyword, WithdrawnKeyword,
};

pub(super) const CONTEST_KINDS: &[SyntaxKind] = &[
    SingleKeyword,
    MultipleKeyword,
    RankedKeyword,
    RetentionKeyword,
];
pub(super) const CONTEST_FIELDS: &[SyntaxKind] = &[SubtitleKeyword, TabulationKeyword, NoteKeyword];
pub(super) const CONTEST_ITEMS: &[SyntaxKind] = &[CampaignKeyword, RecommendKeyword];
pub(super) const CAMPAIGN_FIELDS: &[SyntaxKind] = &[
    WebsiteKeyword,
    NoteKeyword,
    DesignationKeyword,
    PartyKeyword,
];

impl Parser<'_> {
    pub(super) fn contest(&mut self) {
        self.start_node(Contest);
        self.bump();
        self.identifier();
        self.expect(StringLiteral);
        self.contest_kind();
        self.end_of_line();
        self.required_block(CONTEST_FIELDS, CONTEST_ITEMS, Self::contest_item);
        self.finish_node();
    }

    pub(super) fn contest_kind(&mut self) {
        match self.current() {
            Some(SingleKeyword | RetentionKeyword) => {
                self.start_node(ContestKind);
                self.bump();
                self.finish_node();
            }
            Some(MultipleKeyword) => {
                self.start_node(ContestKind);
                self.bump();
                self.expect_value(IntegerLiteral);
                self.finish_node();
            }
            Some(RankedKeyword) => {
                self.start_node(ContestKind);
                self.bump();
                if self.eat(MaxKeyword) {
                    self.expect_value(IntegerLiteral);
                }
                self.finish_node();
            }
            _ if self.at_line_end() => self.error(CONTEST_KINDS),
            _ => self.recover_line(CONTEST_KINDS),
        }
    }

    pub(super) fn contest_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            CampaignKeyword => self.campaign(),
            RecommendKeyword => self.contest_recommend(),
            SubtitleKeyword | TabulationKeyword | NoteKeyword => return self.simple_field(kind),
            _ => return false,
        }
        true
    }

    pub(super) fn campaign(&mut self) {
        self.start_node(Campaign);
        self.bump();
        self.flags();
        if self.at(At) {
            self.member(false);
            self.end_of_line();
            self.block(CAMPAIGN_FIELDS, &[At], Self::campaign_block_item);
        } else if self.at_line_end() {
            self.end_of_line();
            self.required_block(CAMPAIGN_FIELDS, &[At], Self::campaign_block_item);
        } else {
            self.recover_line(&[IncumbentKeyword, WithdrawnKeyword, At, Newline]);
            self.end_of_line();
            self.block(CAMPAIGN_FIELDS, &[At], Self::campaign_block_item);
        }
        self.finish_node();
    }

    pub(super) fn flags(&mut self) {
        let mut remaining = vec![IncumbentKeyword, WithdrawnKeyword];
        while let Some(kind @ (IncumbentKeyword | WithdrawnKeyword)) = self.current() {
            if remaining.contains(&kind) {
                remaining.retain(|k| *k != kind);
                self.flag();
            } else {
                let mut expected = remaining.clone();
                expected.push(At);
                self.misplaced(kind, &expected, |parser, _| {
                    parser.flag();
                    true
                });
            }
        }
    }

    pub(super) fn flag(&mut self) {
        self.start_node(Flag);
        self.bump();
        self.finish_node();
    }

    pub(super) fn member(&mut self, in_block: bool) {
        self.start_node(Member);
        self.identifier();
        self.eat(StringLiteral);
        if self.eat(AsKeyword) {
            self.expect_value(StringLiteral);
        }
        if in_block {
            self.end_of_line();
            self.block(CANDIDATE_FIELDS, &[], Self::candidate_field);
        }
        self.finish_node();
    }

    pub(super) fn campaign_block_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            At => self.member(true),
            PartyKeyword => self.party_reference(),
            WebsiteKeyword | NoteKeyword | DesignationKeyword => return self.simple_field(kind),
            _ => return false,
        }
        true
    }
}
