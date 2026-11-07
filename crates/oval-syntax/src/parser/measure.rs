use super::recommend::RECOMMEND_FIELDS;
use super::{ParseErrorKind, Parser};
use crate::SyntaxKind::{
    self, Comma, DescriptionKeyword, FiscalImpactKeyword, IfNoKeyword, IfYesKeyword, Measure,
    MeasureRecommend, Newline, NoKeyword, NoPositionKeyword, NoneValueKeyword, NoteKeyword,
    OpponentsKeyword, PlacementKeyword, RecommendKeyword, StringListField, StringLiteral,
    SubtitleKeyword, SupportersKeyword, TabulationKeyword, YesKeyword,
};

pub(super) const MEASURE_FIELDS: &[SyntaxKind] = &[
    SubtitleKeyword,
    DescriptionKeyword,
    IfYesKeyword,
    IfNoKeyword,
    PlacementKeyword,
    TabulationKeyword,
    FiscalImpactKeyword,
    SupportersKeyword,
    OpponentsKeyword,
    NoteKeyword,
];
pub(super) const MEASURE_POSITIONS: &[SyntaxKind] = &[YesKeyword, NoKeyword, NoPositionKeyword];

impl Parser<'_> {
    pub(super) fn measure(&mut self) {
        self.start_node(Measure);
        self.bump();
        self.declaration_header();
        self.required_block(MEASURE_FIELDS, &[RecommendKeyword], Self::measure_item);
        self.finish_node();
    }

    pub(super) fn measure_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            RecommendKeyword => self.measure_recommend(),
            SupportersKeyword | OpponentsKeyword => self.string_list_field(),
            _ if MEASURE_FIELDS.contains(&kind) => return self.simple_field(kind),
            _ => return false,
        }
        true
    }

    pub(super) fn string_list_field(&mut self) {
        self.start_node(StringListField);
        self.bump();
        if self.eat(StringLiteral) {
            while self.eat(Comma) {
                self.expect_value(StringLiteral);
            }
            if !self.at_line_end() {
                self.recover_line(&[Comma, Newline]);
            }
        } else if !self.eat(NoneValueKeyword) {
            if self.at_line_end() {
                self.missing(
                    ParseErrorKind::MissingValue,
                    &[StringLiteral, NoneValueKeyword],
                );
            } else {
                self.recover_line(&[StringLiteral, NoneValueKeyword]);
            }
        }
        self.end_of_line();
        self.finish_node();
    }

    pub(super) fn measure_recommend(&mut self) {
        self.start_node(MeasureRecommend);
        self.bump();
        match self.current() {
            Some(YesKeyword | NoKeyword | NoPositionKeyword) => self.bump(),
            _ if self.at_line_end() => {
                self.missing(ParseErrorKind::MissingValue, MEASURE_POSITIONS);
            }
            _ => self.recover_line(MEASURE_POSITIONS),
        }
        self.end_of_line();
        self.block(RECOMMEND_FIELDS, &[], Self::recommend_field);
        self.finish_node();
    }
}
