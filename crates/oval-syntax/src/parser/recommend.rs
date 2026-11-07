use rowan::TextRange;

use super::{ParseError, ParseErrorKind, Parser, text_size};
use crate::SyntaxKind::{
    self, At, ContestRecommend, Indent, JustificationKeyword, Newline, NoPositionKeyword,
    NoteKeyword, Oppose, OpposeKeyword, RecommendKeyword, StringLiteral, Support, SupportKeyword,
};

pub(super) const RECOMMEND_FIELDS: &[SyntaxKind] = &[JustificationKeyword, NoteKeyword];

impl Parser<'_> {
    pub(super) fn contest_recommend(&mut self) {
        self.start_node(ContestRecommend);
        let start = self.offset;
        self.bump();
        let keyword_end = self.offset;
        if self.eat(NoPositionKeyword) {
            self.end_of_line();
            self.block(RECOMMEND_FIELDS, &[], Self::recommend_field);
        } else if self.at_line_end() {
            self.end_of_line();
            if self.at(Indent) {
                let stances = self.stances;
                self.block(
                    RECOMMEND_FIELDS,
                    &[SupportKeyword, OpposeKeyword],
                    Self::recommend_item,
                );
                if self.stances == stances {
                    self.errors.push(ParseError {
                        kind: ParseErrorKind::EmptyRecommendation,
                        expected: vec![SupportKeyword, OpposeKeyword, NoPositionKeyword],
                        found: Some(RecommendKeyword),
                        range: TextRange::new(text_size(start), text_size(keyword_end)),
                    });
                }
            } else {
                self.error_at(self.offset, ParseErrorKind::MissingBlock, &[Indent]);
            }
        } else {
            self.recover_line(&[NoPositionKeyword, Newline]);
            self.end_of_line();
            self.required_block(
                RECOMMEND_FIELDS,
                &[SupportKeyword, OpposeKeyword],
                Self::recommend_item,
            );
        }
        self.finish_node();
    }

    pub(super) fn recommend_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            SupportKeyword => self.references(Support),
            OpposeKeyword => self.references(Oppose),
            _ => return self.recommend_field(kind),
        }
        true
    }

    pub(super) fn recommend_field(&mut self, kind: SyntaxKind) -> bool {
        matches!(kind, JustificationKeyword | NoteKeyword) && self.simple_field(kind)
    }

    pub(super) fn references(&mut self, node: SyntaxKind) {
        self.start_node(node);
        self.bump();
        self.stances += 1;
        if !self.at(At) {
            if self.at_line_end() {
                self.missing(ParseErrorKind::MissingIdentifier, &[At]);
            } else if self.at(StringLiteral) {
                self.missing(ParseErrorKind::MissingIdentifier, &[At]);
                self.skip_token();
            } else {
                self.recover_line(&[At]);
            }
        }
        while self.at(At) {
            self.identifier();
        }
        if !self.at_line_end() {
            self.recover_line(&[At, Newline]);
        }
        self.expect(Newline);
        self.finish_node();
    }
}
