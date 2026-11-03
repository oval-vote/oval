//! Builds a lossless syntax tree from source text.

use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

use crate::SyntaxKind::{
    self, AbbreviationKeyword, AsKeyword, Ballot, BallotKeyword, Block, Campaign, CampaignKeyword,
    Candidate, CandidateKeyword, Colon, Comma, Contest, ContestKeyword, ContestKind,
    ContestRecommend, DateField, DateKeyword, DateLiteral, Dedent, DescriptionKeyword,
    DesignationKeyword, Document, ElectionKeyword, EligibilityKeyword, ErrorNode,
    FiscalImpactKeyword, Flag, GuideKeyword, Identifier, IfNoKeyword, IfYesKeyword,
    IncumbentKeyword, Indent, IntegerLiteral, JustificationKeyword, MarkdownBlock, MarkdownField,
    MaxKeyword, Measure, MeasureKeyword, MeasureRecommend, Member, MultipleKeyword, Newline,
    NoKeyword, NoPositionKeyword, NoneValueKeyword, NoteKeyword, OpponentsKeyword, Oppose,
    OpposeKeyword, Party, PartyKeyword, PartyReference, PlacementKeyword, RankedKeyword,
    RecommendKeyword, RetentionKeyword, Section, SectionKeyword, SingleKeyword, StringField,
    StringListField, StringLiteral, SubtitleKeyword, Support, SupportKeyword, SupportersKeyword,
    TabulationKeyword, UrlField, UrlLiteral, WebsiteKeyword, WithdrawnKeyword, YesKeyword,
};
use crate::SyntaxNode;
use crate::lexer::{LexError, Token, lex};

/// An error found while parsing tokens into a syntax tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    /// The kinds that would have been accepted. Empty when nothing was acceptable.
    pub expected: Vec<SyntaxKind>,

    /// The kind actually found, or `None` at end of input.
    pub found: Option<SyntaxKind>,

    /// The skipped text, or an empty range where something was missing.
    pub range: TextRange,
}

/// Any error found while turning source text into a tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SyntaxError {
    /// The lexer could not tokenize some text.
    Lex(LexError),

    /// The parser could not fit the tokens to the grammar.
    Parse(ParseError),
}

impl SyntaxError {
    /// The range of source text where the error occurred.
    pub fn range(&self) -> TextRange {
        match self {
            Self::Lex(error) => error.range,
            Self::Parse(error) => error.range,
        }
    }
}

/// The result of parsing source text into a syntax tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    root: GreenNode,
    errors: Vec<SyntaxError>,
}

impl ParseResult {
    /// The root of the syntax tree.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.root.clone())
    }

    /// All errors, in source order.
    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }
}

/// Parses `source` into a tree.
pub fn parse(source: &str) -> ParseResult {
    let lexed = lex(source);
    let mut parser = Parser {
        source,
        tokens: lexed.tokens,
        pos: 0,
        offset: 0,
        stances: 0,
        builder: GreenNodeBuilder::new(),
        errors: Vec::new(),
    };
    parser.document();
    let mut errors: Vec<SyntaxError> = lexed.errors.into_iter().map(SyntaxError::Lex).collect();
    errors.extend(parser.errors.into_iter().map(SyntaxError::Parse));
    errors.sort_by_key(|error| error.range().start());
    ParseResult {
        root: parser.builder.finish(),
        errors,
    }
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    offset: usize,
    stances: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

impl Parser<'_> {
    fn document(&mut self) {
        self.builder.start_node(Document.into());
        let mut remaining = TOP_LEVEL.to_vec();
        while let Some(kind) = self.current() {
            match kind {
                GuideKeyword | ElectionKeyword | DateKeyword | NoteKeyword => {
                    self.field_once(kind, &mut remaining, Self::simple_field);
                }
                PartyKeyword | CandidateKeyword => {
                    if remaining.contains(&kind) {
                        remaining.retain(|k| !GUIDE_FIELDS.contains(k));
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

    fn declaration(&mut self, kind: SyntaxKind) -> bool {
        if kind == PartyKeyword {
            self.party();
        } else {
            self.candidate();
        }
        true
    }

    fn party(&mut self) {
        self.start_node(Party);
        self.bump();
        self.declaration_header();
        self.block(PARTY_FIELDS, &[], Self::simple_field);
        self.finish_node();
    }

    fn candidate(&mut self) {
        self.start_node(Candidate);
        self.bump();
        self.declaration_header();
        self.block(CANDIDATE_FIELDS, &[], Self::candidate_field);
        self.finish_node();
    }

    fn candidate_field(&mut self, kind: SyntaxKind) -> bool {
        if kind == PartyKeyword {
            self.party_reference();
            true
        } else {
            self.simple_field(kind)
        }
    }

    fn ballot(&mut self) {
        self.start_node(Ballot);
        self.bump();
        self.declaration_header();
        self.required_block(BALLOT_FIELDS, BALLOT_ITEMS, Self::ballot_item);
        self.finish_node();
    }

    fn ballot_item(&mut self, kind: SyntaxKind) -> bool {
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

    fn section(&mut self) {
        self.start_node(Section);
        self.bump();
        self.expect_value(StringLiteral);
        self.end_of_line();
        self.required_block(SECTION_FIELDS, SECTION_ITEMS, Self::section_item);
        self.finish_node();
    }

    fn section_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            ContestKeyword => self.contest(),
            MeasureKeyword => self.measure(),
            NoteKeyword => return self.simple_field(kind),
            _ => return false,
        }
        true
    }

    fn measure(&mut self) {
        self.start_node(Measure);
        self.bump();
        self.declaration_header();
        self.required_block(MEASURE_FIELDS, &[RecommendKeyword], Self::measure_item);
        self.finish_node();
    }

    fn measure_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            RecommendKeyword => self.measure_recommend(),
            SupportersKeyword | OpponentsKeyword => self.string_list_field(),
            _ if MEASURE_FIELDS.contains(&kind) => return self.simple_field(kind),
            _ => return false,
        }
        true
    }

    fn string_list_field(&mut self) {
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
                self.error(&[StringLiteral, NoneValueKeyword]);
            } else {
                self.recover_line(&[StringLiteral, NoneValueKeyword]);
            }
        }
        self.end_of_line();
        self.finish_node();
    }

    fn measure_recommend(&mut self) {
        self.start_node(MeasureRecommend);
        self.bump();
        match self.current() {
            Some(YesKeyword | NoKeyword | NoPositionKeyword) => self.bump(),
            _ if self.at_line_end() => self.error(MEASURE_POSITIONS),
            _ => self.recover_line(MEASURE_POSITIONS),
        }
        self.end_of_line();
        self.block(RECOMMEND_FIELDS, &[], Self::recommend_field);
        self.finish_node();
    }

    fn contest(&mut self) {
        self.start_node(Contest);
        self.bump();
        self.expect(Identifier);
        self.expect(StringLiteral);
        self.contest_kind();
        self.end_of_line();
        self.required_block(CONTEST_FIELDS, CONTEST_ITEMS, Self::contest_item);
        self.finish_node();
    }

    fn contest_kind(&mut self) {
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

    fn contest_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            CampaignKeyword => self.campaign(),
            RecommendKeyword => self.contest_recommend(),
            SubtitleKeyword | TabulationKeyword | NoteKeyword => return self.simple_field(kind),
            _ => return false,
        }
        true
    }

    fn contest_recommend(&mut self) {
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
                        expected: vec![SupportKeyword, OpposeKeyword, NoPositionKeyword],
                        found: Some(RecommendKeyword),
                        range: TextRange::new(text_size(start), text_size(keyword_end)),
                    });
                }
            } else {
                self.error(&[Indent]);
            }
        } else {
            self.recover_line(&[NoPositionKeyword, Newline]);
            self.end_of_line();
            self.block(
                RECOMMEND_FIELDS,
                &[SupportKeyword, OpposeKeyword],
                Self::recommend_item,
            );
        }
        self.finish_node();
    }

    fn recommend_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            SupportKeyword => self.references(Support),
            OpposeKeyword => self.references(Oppose),
            _ => return self.recommend_field(kind),
        }
        true
    }

    fn recommend_field(&mut self, kind: SyntaxKind) -> bool {
        matches!(kind, JustificationKeyword | NoteKeyword) && self.simple_field(kind)
    }

    fn references(&mut self, node: SyntaxKind) {
        self.start_node(node);
        self.bump();
        self.stances += 1;
        if !self.at(Identifier) {
            if self.at_line_end() {
                self.error(&[Identifier]);
            } else {
                self.recover_line(&[Identifier]);
            }
        }
        while self.eat(Identifier) {}
        if !self.at_line_end() {
            self.recover_line(&[Identifier, Newline]);
        }
        self.expect(Newline);
        self.finish_node();
    }

    fn campaign(&mut self) {
        self.start_node(Campaign);
        self.bump();
        self.flags();
        if self.at(Identifier) {
            self.member(false);
            self.end_of_line();
            self.block(CAMPAIGN_FIELDS, &[Identifier], Self::campaign_block_item);
        } else if self.at_line_end() {
            self.end_of_line();
            self.required_block(CAMPAIGN_FIELDS, &[Identifier], Self::campaign_block_item);
        } else {
            self.recover_line(&[IncumbentKeyword, WithdrawnKeyword, Identifier, Newline]);
            self.end_of_line();
            self.block(CAMPAIGN_FIELDS, &[Identifier], Self::campaign_block_item);
        }
        self.finish_node();
    }

    fn flags(&mut self) {
        let mut remaining = vec![IncumbentKeyword, WithdrawnKeyword];
        while let Some(kind @ (IncumbentKeyword | WithdrawnKeyword)) = self.current() {
            if remaining.contains(&kind) {
                remaining.retain(|k| *k != kind);
                self.flag();
            } else {
                let mut expected = remaining.clone();
                expected.push(Identifier);
                self.misplaced(kind, &expected, |parser, _| {
                    parser.flag();
                    true
                });
            }
        }
    }

    fn flag(&mut self) {
        self.start_node(Flag);
        self.bump();
        self.finish_node();
    }

    fn member(&mut self, in_block: bool) {
        self.start_node(Member);
        self.bump();
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

    fn campaign_block_item(&mut self, kind: SyntaxKind) -> bool {
        match kind {
            Identifier => self.member(true),
            PartyKeyword => self.party_reference(),
            WebsiteKeyword | NoteKeyword | DesignationKeyword => return self.simple_field(kind),
            _ => return false,
        }
        true
    }

    fn party_reference(&mut self) {
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

    fn declaration_header(&mut self) {
        self.expect(Identifier);
        self.expect_value(StringLiteral);
        self.end_of_line();
    }

    fn required_block(
        &mut self,
        fields: &[SyntaxKind],
        items: &[SyntaxKind],
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        if self.at(Indent) {
            self.block(fields, items, parse);
        } else {
            self.error(&[Indent]);
        }
    }

    fn block(
        &mut self,
        fields: &[SyntaxKind],
        items: &[SyntaxKind],
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        if !self.at(Indent) {
            return;
        }
        self.start_node(Block);
        self.bump();
        let mut remaining = fields.to_vec();
        while let Some(kind) = self.current() {
            if kind == Dedent {
                break;
            }
            if fields.contains(&kind) {
                self.field_once(kind, &mut remaining, parse);
            } else if !(items.contains(&kind) && parse(self, kind)) {
                let expected: Vec<SyntaxKind> = remaining.iter().chain(items).copied().collect();
                self.recover_item(&expected);
            }
        }
        self.expect(Dedent);
        self.finish_node();
    }

    fn field_once(
        &mut self,
        kind: SyntaxKind,
        remaining: &mut Vec<SyntaxKind>,
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        if remaining.contains(&kind) {
            remaining.retain(|k| *k != kind);
            parse(self, kind);
        } else {
            self.misplaced(kind, remaining, parse);
        }
    }

    fn misplaced(
        &mut self,
        kind: SyntaxKind,
        expected: &[SyntaxKind],
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        self.start_node(ErrorNode);
        let start = self.offset;
        parse(self, kind);
        self.finish_node();
        self.errors.push(ParseError {
            expected: expected.to_vec(),
            found: Some(kind),
            range: TextRange::new(text_size(start), text_size(self.offset)),
        });
    }

    fn simple_field(&mut self, kind: SyntaxKind) -> bool {
        let (node, value) = match kind {
            DateKeyword => (DateField, DateLiteral),
            WebsiteKeyword => (UrlField, UrlLiteral),
            NoteKeyword | JustificationKeyword | DescriptionKeyword | IfYesKeyword
            | IfNoKeyword => (MarkdownField, MarkdownBlock),
            _ => (StringField, StringLiteral),
        };
        self.start_node(node);
        self.bump();
        if value == MarkdownBlock {
            self.markdown_value();
        } else {
            self.expect_value(value);
            self.end_of_line();
        }
        self.finish_node();
        true
    }

    fn markdown_value(&mut self) {
        if !self.expect(Colon) {
            self.end_of_line();
            return;
        }
        if self.at_line_end() {
            self.end_of_line();
            if !self.eat(MarkdownBlock) {
                self.error(&[MarkdownBlock]);
            }
        } else {
            self.end_of_line();
        }
    }

    fn expect_value(&mut self, kind: SyntaxKind) {
        if self.eat(kind) {
            return;
        }
        if self.at_line_end() {
            self.error(&[kind]);
        } else {
            self.recover_line(&[kind]);
        }
    }

    fn end_of_line(&mut self) {
        if !self.at_line_end() {
            self.recover_line(&[Newline]);
        }
        self.expect(Newline);
    }

    fn at_line_end(&self) -> bool {
        matches!(self.current(), None | Some(Newline | Indent | Dedent))
    }

    fn recover_line(&mut self, expected: &[SyntaxKind]) {
        self.start_node(ErrorNode);
        let start = self.offset;
        let found = self.current();
        while !self.at_line_end() {
            self.bump();
        }
        self.finish_node();
        self.errors.push(ParseError {
            expected: expected.to_vec(),
            found,
            range: TextRange::new(text_size(start), text_size(self.offset)),
        });
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.error(&[kind]);
        false
    }

    fn error(&mut self, expected: &[SyntaxKind]) {
        let offset = self.tokens[self.pos..]
            .iter()
            .take_while(|token| token.kind.is_trivia())
            .map(|token| usize::from(token.len))
            .sum::<usize>()
            + self.offset;
        self.errors.push(ParseError {
            expected: expected.to_vec(),
            found: self.current(),
            range: TextRange::empty(text_size(offset)),
        });
    }

    fn current(&self) -> Option<SyntaxKind> {
        self.tokens[self.pos..]
            .iter()
            .map(|token| token.kind)
            .find(|kind| !kind.is_trivia())
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == Some(kind)
    }

    fn bump(&mut self) {
        self.eat_trivia();
        self.bump_raw();
    }

    fn bump_raw(&mut self) {
        let token = self.tokens[self.pos];
        let end = self.offset + usize::from(token.len);
        self.builder
            .token(token.kind.into(), &self.source[self.offset..end]);
        self.offset = end;
        self.pos += 1;
    }

    fn eat_trivia(&mut self) {
        while self
            .tokens
            .get(self.pos)
            .is_some_and(|token| token.kind.is_trivia())
        {
            self.bump_raw();
        }
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        self.builder.start_node(kind.into());
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    fn recover_item(&mut self, expected: &[SyntaxKind]) {
        self.start_node(ErrorNode);
        let start = self.offset;
        let found = self.current();
        if self.at(Indent) {
            self.skip_block();
        } else {
            while !matches!(self.current(), None | Some(Newline | Indent | Dedent)) {
                self.bump();
            }
            if self.at(Newline) {
                self.bump();
            }
            if self.at(Indent) {
                self.skip_block();
            }
            if self.offset == start && self.current().is_some() {
                self.bump();
            }
        }
        self.finish_node();
        self.errors.push(ParseError {
            expected: expected.to_vec(),
            found,
            range: TextRange::new(text_size(start), text_size(self.offset)),
        });
    }

    fn skip_block(&mut self) {
        let mut depth = 0usize;
        while let Some(kind) = self.current() {
            self.bump();
            match kind {
                Indent => depth += 1,
                Dedent => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}

const GUIDE_FIELDS: &[SyntaxKind] = &[GuideKeyword, ElectionKeyword, DateKeyword, NoteKeyword];
const TOP_LEVEL: &[SyntaxKind] = &[
    GuideKeyword,
    ElectionKeyword,
    DateKeyword,
    NoteKeyword,
    PartyKeyword,
    CandidateKeyword,
    BallotKeyword,
];
const PARTY_FIELDS: &[SyntaxKind] = &[AbbreviationKeyword, WebsiteKeyword, NoteKeyword];
const CANDIDATE_FIELDS: &[SyntaxKind] = &[DesignationKeyword, PartyKeyword, NoteKeyword];
const BALLOT_FIELDS: &[SyntaxKind] = &[EligibilityKeyword, NoteKeyword];
const BALLOT_ITEMS: &[SyntaxKind] = &[
    SectionKeyword,
    ContestKeyword,
    MeasureKeyword,
    BallotKeyword,
];
const SECTION_FIELDS: &[SyntaxKind] = &[NoteKeyword];
const SECTION_ITEMS: &[SyntaxKind] = &[ContestKeyword, MeasureKeyword];
const CONTEST_KINDS: &[SyntaxKind] = &[
    SingleKeyword,
    MultipleKeyword,
    RankedKeyword,
    RetentionKeyword,
];
const CONTEST_FIELDS: &[SyntaxKind] = &[SubtitleKeyword, TabulationKeyword, NoteKeyword];
const CONTEST_ITEMS: &[SyntaxKind] = &[CampaignKeyword, RecommendKeyword];
const RECOMMEND_FIELDS: &[SyntaxKind] = &[JustificationKeyword, NoteKeyword];
const MEASURE_FIELDS: &[SyntaxKind] = &[
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
const MEASURE_POSITIONS: &[SyntaxKind] = &[YesKeyword, NoKeyword, NoPositionKeyword];
const CAMPAIGN_FIELDS: &[SyntaxKind] = &[
    WebsiteKeyword,
    NoteKeyword,
    DesignationKeyword,
    PartyKeyword,
];

fn text_size(offset: usize) -> TextSize {
    TextSize::try_from(offset).expect("source longer than 4 GiB")
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use rowan::{NodeOrToken, TextRange, TextSize};

    use super::{ParseError, SyntaxError, parse};
    use crate::SyntaxKind;
    use crate::lexer::LexErrorReason;

    const SAMPLE: &str = "\
guide \"Franklin Voter Guide\"
election \"November 3, 2026 General Election\"
date 2026-11-03

note:
  Written by the Franklin Civic Alliance.

  Second paragraph.

party @dem \"Democratic\"
  abbreviation \"D\"

ballot @franklin \"State of Franklin\"
  section \"Statewide Offices\"
    contest @gov \"Governor\" single
      campaign incumbent @jlee \"Jordan Lee\"
        party @dem
        website https://leepatel2026.example
      campaign @rkim \"Riley Kim\" # trailing comment
      recommend
        support @jlee
        oppose @rkim
        justification:
          Lee delivered.
  measure @prop1 \"Proposition 1\"
    supporters \"A\", \"B\"
    opponents none
    recommend yes
";

    fn dump(node: &crate::SyntaxNode) -> String {
        fn go(element: rowan::SyntaxElement<crate::OvalLanguage>, depth: usize, out: &mut String) {
            let indent = "  ".repeat(depth);
            match element {
                NodeOrToken::Node(node) => {
                    writeln!(out, "{indent}{:?}", node.kind()).unwrap();
                    for child in node.children_with_tokens() {
                        go(child, depth + 1, out);
                    }
                }
                NodeOrToken::Token(token) => {
                    if !token.kind().is_trivia() {
                        writeln!(out, "{indent}{:?} {:?}", token.kind(), token.text()).unwrap();
                    }
                }
            }
        }
        let mut out = String::new();
        go(NodeOrToken::Node(node.clone()), 0, &mut out);
        out
    }

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    fn span(source: &str, needle: &str) -> TextRange {
        let start = source.find(needle).expect("needle in source");
        let start = u32::try_from(start).unwrap();
        let len = u32::try_from(needle.len()).unwrap();
        range(start, start + len)
    }

    fn in_ballot(body: &str) -> String {
        let mut out = String::from("ballot @b \"B\"\n");
        for line in body.lines() {
            if line.is_empty() {
                out.push('\n');
            } else {
                out.push_str("  ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }

    fn first_contest(parse: &super::ParseResult) -> crate::SyntaxNode {
        first(parse, SyntaxKind::Contest)
    }

    fn first(parse: &super::ParseResult, kind: SyntaxKind) -> crate::SyntaxNode {
        parse
            .syntax()
            .descendants()
            .find(|n| n.kind() == kind)
            .unwrap_or_else(|| panic!("a {kind:?}"))
    }

    fn in_contest(body: &str) -> String {
        let mut inner = String::from("contest @c \"C\" single\n");
        for line in body.lines() {
            if !line.is_empty() {
                inner.push_str("  ");
                inner.push_str(line);
            }
            inner.push('\n');
        }
        in_ballot(&inner)
    }

    #[test]
    fn the_tree_reproduces_the_source_exactly() {
        for source in [
            "",
            "   ",
            "\n\n",
            "# only a comment",
            "guide \"x\"",
            "guide \"x\"\n",
            "  indented first line\n",
            "a\n  b\n    c\nd",
            "note:\n  body\n\n  more\n",
            "\u{FEFF}guide \"x\"\r\n\tbad\r\n",
            "\"unterminated\n@9 12abc https://\n",
            SAMPLE,
        ] {
            let parse = parse(source);
            assert_eq!(parse.syntax().text(), source, "round trip of {source:?}");
            assert_eq!(parse.syntax().kind(), SyntaxKind::Document);
        }
    }

    const TOP_LEVEL: [SyntaxKind; 7] = [
        SyntaxKind::GuideKeyword,
        SyntaxKind::ElectionKeyword,
        SyntaxKind::DateKeyword,
        SyntaxKind::NoteKeyword,
        SyntaxKind::PartyKeyword,
        SyntaxKind::CandidateKeyword,
        SyntaxKind::BallotKeyword,
    ];

    #[test]
    fn guide_fields_from_the_spec_example() {
        let source = "\
guide \"Franklin Voter Guide\"
election \"November 3, 2026 General Election\"
date 2026-11-03

note:
  Written by the Franklin Civic Alliance.
";
        let parse = parse(source);
        assert_eq!(parse.errors(), []);
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  StringField
    GuideKeyword \"guide\"
    StringLiteral \"\\\"Franklin Voter Guide\\\"\"
    Newline \"\\n\"
  StringField
    ElectionKeyword \"election\"
    StringLiteral \"\\\"November 3, 2026 General Election\\\"\"
    Newline \"\\n\"
  DateField
    DateKeyword \"date\"
    DateLiteral \"2026-11-03\"
    Newline \"\\n\"
  MarkdownField
    NoteKeyword \"note\"
    Colon \":\"
    Newline \"\\n\"
    MarkdownBlock \"  Written by the Franklin Civic Alliance.\\n\"
"
        );
    }

    #[test]
    fn guide_fields_may_appear_in_any_order() {
        let parse = parse("date 2026-11-03\nnote:\n  x\nguide \"g\"\nelection \"e\"");
        assert_eq!(parse.errors(), []);
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::DateField,
                SyntaxKind::MarkdownField,
                SyntaxKind::StringField,
                SyntaxKind::StringField
            ]
        );
    }

    #[test]
    fn a_missing_value_is_reported_where_it_should_be() {
        let parse = parse("guide\n");
        assert_eq!(
            dump(&parse.syntax()),
            "Document\n  StringField\n    GuideKeyword \"guide\"\n    Newline \"\\n\"\n"
        );
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::StringLiteral],
                found: Some(SyntaxKind::Newline),
                range: range(5, 5),
            })]
        );
    }

    #[test]
    fn a_wrong_value_is_one_error_covering_the_rest_of_the_line() {
        let parse = parse("guide 2026-11-03 yes\n");
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  StringField
    GuideKeyword \"guide\"
    ErrorNode
      DateLiteral \"2026-11-03\"
      YesKeyword \"yes\"
    Newline \"\\n\"
"
        );
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::StringLiteral],
                found: Some(SyntaxKind::DateLiteral),
                range: range(6, 20),
            })]
        );
    }

    #[test]
    fn extra_tokens_after_a_value_are_an_error_node_inside_the_field() {
        let parse = parse("guide \"x\" \"y\"\n");
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  StringField
    GuideKeyword \"guide\"
    StringLiteral \"\\\"x\\\"\"
    ErrorNode
      StringLiteral \"\\\"y\\\"\"
    Newline \"\\n\"
"
        );
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::Newline],
                found: Some(SyntaxKind::StringLiteral),
                range: range(10, 13),
            })]
        );
    }

    #[test]
    fn a_repeated_field_is_an_error_node_and_names_what_was_still_allowed() {
        let parse = parse("guide \"a\"\ndate 2026-11-03\nguide \"b\"\n");
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::StringField,
                SyntaxKind::DateField,
                SyntaxKind::ErrorNode
            ]
        );
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![
                    SyntaxKind::ElectionKeyword,
                    SyntaxKind::NoteKeyword,
                    SyntaxKind::PartyKeyword,
                    SyntaxKind::CandidateKeyword,
                    SyntaxKind::BallotKeyword,
                ],
                found: Some(SyntaxKind::GuideKeyword),
                range: range(26, 36),
            })]
        );
    }

    #[test]
    fn a_markdown_field_needs_a_colon_a_newline_and_a_body() {
        let result = parse("note\n");
        assert_eq!(result.errors()[0].range(), range(4, 4));
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.expected == [SyntaxKind::Colon] && e.found == Some(SyntaxKind::Newline)
        ));

        let result = parse("note: yes\n");
        assert_eq!(
            dump(&result.syntax()),
            "\
Document
  MarkdownField
    NoteKeyword \"note\"
    Colon \":\"
    ErrorNode
      YesKeyword \"yes\"
    Newline \"\\n\"
"
        );
        assert_eq!(result.errors().len(), 1);

        let result = parse("note:\nguide \"g\"\n");
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::MarkdownBlock],
                found: Some(SyntaxKind::GuideKeyword),
                range: range(6, 6),
            })]
        );
        let kinds: Vec<SyntaxKind> = result.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(kinds, [SyntaxKind::MarkdownField, SyntaxKind::StringField]);
    }

    #[test]
    fn an_unknown_top_level_line_names_everything_that_could_start_one() {
        let parse = parse("contest @c \"C\" single\n");
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: TOP_LEVEL.to_vec(),
                found: Some(SyntaxKind::ContestKeyword),
                range: range(0, 22),
            })]
        );
    }

    #[test]
    fn an_unexpected_block_after_a_field_is_one_error() {
        let parse = parse("guide \"x\"\n  note:\n    body\n");
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(kinds, [SyntaxKind::StringField, SyntaxKind::ErrorNode]);
        assert_eq!(parse.errors().len(), 1);
        assert!(
            matches!(&parse.errors()[0], SyntaxError::Parse(e) if e.found == Some(SyntaxKind::Indent))
        );
    }

    #[test]
    fn impossible_dates_are_left_to_validation() {
        assert_eq!(parse("date 2026-13-45\n").errors(), []);
    }

    #[test]
    fn an_error_item_swallows_its_own_block_and_nothing_more() {
        let source = "contest @x\n  campaign @a\n  recommend\n    support @a\nmeasure @m\n";
        let parse = parse(source);
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  ErrorNode
    ContestKeyword \"contest\"
    Identifier \"@x\"
    Newline \"\\n\"
    Indent \"\"
    CampaignKeyword \"campaign\"
    Identifier \"@a\"
    Newline \"\\n\"
    RecommendKeyword \"recommend\"
    Newline \"\\n\"
    Indent \"\"
    SupportKeyword \"support\"
    Identifier \"@a\"
    Newline \"\\n\"
    Dedent \"\"
    Dedent \"\"
  ErrorNode
    MeasureKeyword \"measure\"
    Identifier \"@m\"
    Newline \"\\n\"
"
        );
        assert_eq!(parse.errors().len(), 2);
        assert_eq!(parse.errors()[0].range(), range(0, 52));
        assert_eq!(parse.errors()[1].range(), range(52, 63));
    }

    #[test]
    fn an_indented_first_line_is_one_error_item() {
        let parse = parse("  yes\n  no\nyes");
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  ErrorNode
    Indent \"\"
    YesKeyword \"yes\"
    Newline \"\\n\"
    NoKeyword \"no\"
    Newline \"\\n\"
    Dedent \"\"
  ErrorNode
    YesKeyword \"yes\"
    Newline \"\"
"
        );
    }

    #[test]
    fn trivia_is_attached_to_the_enclosing_node() {
        let parse = parse("# lead\n\nyes # trail\n\n# tail\n");
        let root = parse.syntax();
        let error_node = root.first_child().expect("one error item");
        assert_eq!(error_node.text(), "yes # trail\n");
        assert_eq!(root.text(), "# lead\n\nyes # trail\n\n# tail\n");
    }

    #[test]
    fn lexer_errors_are_forwarded_in_source_order() {
        let parse = parse("\tyes\n@9\n");
        let kinds: Vec<&str> = parse
            .errors()
            .iter()
            .map(|error| match error {
                SyntaxError::Lex(e) if e.reason == LexErrorReason::IllegalTab => "tab",
                SyntaxError::Lex(e) if e.reason == LexErrorReason::MalformedIdentifier => "ident",
                SyntaxError::Lex(_) => "other lex",
                SyntaxError::Parse(_) => "parse",
            })
            .collect();
        assert_eq!(kinds, ["tab", "parse", "ident", "parse"]);
    }

    #[test]
    fn party_declaration_from_the_spec_example() {
        let parse = parse("party @dem \"Democratic\"\n  abbreviation \"D\"\n");
        assert_eq!(parse.errors(), []);
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  Party
    PartyKeyword \"party\"
    Identifier \"@dem\"
    StringLiteral \"\\\"Democratic\\\"\"
    Newline \"\\n\"
    Block
      Indent \"\"
      StringField
        AbbreviationKeyword \"abbreviation\"
        StringLiteral \"\\\"D\\\"\"
        Newline \"\\n\"
      Dedent \"\"
"
        );
    }

    #[test]
    fn party_fields_in_any_order_and_a_party_without_a_block() {
        let parse = parse(
            "party @a \"A\"\n  note:\n    x\n  website https://a.example\n  abbreviation \"A\"\nparty @b \"B\"\n",
        );
        assert_eq!(parse.errors(), []);
        let root = parse.syntax();
        let parties: Vec<_> = root.children().collect();
        assert_eq!(parties.len(), 2);
        let block = parties[0].last_child().expect("block");
        assert_eq!(block.kind(), SyntaxKind::Block);
        let kinds: Vec<SyntaxKind> = block.children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::MarkdownField,
                SyntaxKind::UrlField,
                SyntaxKind::StringField
            ]
        );
        assert_eq!(parties[1].children().count(), 0);
    }

    #[test]
    fn candidate_declaration_from_the_spec_example() {
        let parse = parse(
            "candidate @nreyes \"Nadia Reyes\"\n  designation \"United States Senator\"\n  party @dem\n  note:\n    Appointed in January 2025.\n",
        );
        assert_eq!(parse.errors(), []);
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  Candidate
    CandidateKeyword \"candidate\"
    Identifier \"@nreyes\"
    StringLiteral \"\\\"Nadia Reyes\\\"\"
    Newline \"\\n\"
    Block
      Indent \"\"
      StringField
        DesignationKeyword \"designation\"
        StringLiteral \"\\\"United States Senator\\\"\"
        Newline \"\\n\"
      PartyReference
        PartyKeyword \"party\"
        Identifier \"@dem\"
        Newline \"\\n\"
      MarkdownField
        NoteKeyword \"note\"
        Colon \":\"
        Newline \"\\n\"
        MarkdownBlock \"    Appointed in January 2025.\\n\"
      Dedent \"\"
"
        );
    }

    #[test]
    fn a_party_may_be_declared_inline_on_a_candidate() {
        let parse = parse("candidate @c \"C\"\n  party @green \"Green\"\n    abbreviation \"G\"\n");
        assert_eq!(parse.errors(), []);
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  Candidate
    CandidateKeyword \"candidate\"
    Identifier \"@c\"
    StringLiteral \"\\\"C\\\"\"
    Newline \"\\n\"
    Block
      Indent \"\"
      PartyReference
        PartyKeyword \"party\"
        Identifier \"@green\"
        StringLiteral \"\\\"Green\\\"\"
        Newline \"\\n\"
        Block
          Indent \"\"
          StringField
            AbbreviationKeyword \"abbreviation\"
            StringLiteral \"\\\"G\\\"\"
            Newline \"\\n\"
          Dedent \"\"
      Dedent \"\"
"
        );
    }

    #[test]
    fn a_party_reference_takes_no_block() {
        let parse = parse("candidate @c \"C\"\n  party @d\n    abbreviation \"D\"\n");
        assert_eq!(parse.errors().len(), 1);
        assert!(matches!(
            &parse.errors()[0],
            SyntaxError::Parse(e) if e.found == Some(SyntaxKind::Indent)
                && e.expected == [SyntaxKind::DesignationKeyword, SyntaxKind::NoteKeyword]
        ));
    }

    #[test]
    fn declaration_headers_report_each_missing_part_once() {
        let result = parse("party \"D\"\n");
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::Identifier],
                found: Some(SyntaxKind::StringLiteral),
                range: range(6, 6),
            })]
        );
        let result = parse("party @d\n");
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::StringLiteral],
                found: Some(SyntaxKind::Newline),
                range: range(8, 8),
            })]
        );
    }

    #[test]
    fn unknown_and_repeated_fields_inside_a_block() {
        let parse = parse(
            "party @d \"D\"\n  designation \"x\"\n  abbreviation \"A\"\n  abbreviation \"B\"\n",
        );
        let block = parse.syntax().first_child().unwrap().last_child().unwrap();
        let kinds: Vec<SyntaxKind> = block.children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::ErrorNode,
                SyntaxKind::StringField,
                SyntaxKind::ErrorNode
            ]
        );
        assert_eq!(
            parse.errors(),
            [
                SyntaxError::Parse(ParseError {
                    expected: vec![
                        SyntaxKind::AbbreviationKeyword,
                        SyntaxKind::WebsiteKeyword,
                        SyntaxKind::NoteKeyword,
                    ],
                    found: Some(SyntaxKind::DesignationKeyword),
                    range: range(15, 31),
                }),
                SyntaxError::Parse(ParseError {
                    expected: vec![SyntaxKind::WebsiteKeyword, SyntaxKind::NoteKeyword],
                    found: Some(SyntaxKind::AbbreviationKeyword),
                    range: range(52, 69),
                }),
            ]
        );
    }

    #[test]
    fn guide_fields_must_precede_declarations() {
        let parse = parse("party @d \"D\"\nguide \"g\"\n");
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(kinds, [SyntaxKind::Party, SyntaxKind::ErrorNode]);
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![
                    SyntaxKind::PartyKeyword,
                    SyntaxKind::CandidateKeyword,
                    SyntaxKind::BallotKeyword,
                ],
                found: Some(SyntaxKind::GuideKeyword),
                range: range(13, 23),
            })]
        );
    }

    #[test]
    fn a_broken_declaration_does_not_damage_its_neighbors() {
        let parse = parse(
            "party @a \"A\"\n  abbreviation 3\n  yes\ncandidate @c \"C\"\n  designation \"d\"\n",
        );
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(kinds, [SyntaxKind::Party, SyntaxKind::Candidate]);
        assert_eq!(parse.errors().len(), 2);
    }

    #[test]
    fn ballot_with_fields_a_section_and_a_sub_ballot() {
        let source = "\
ballot @franklin \"State of Franklin\"
  eligibility \"Registered voters of Franklin\"
  section \"Statewide Offices\"
    note:
      Top of the ticket.
  ballot @ashford \"City of Ashford\"
    note:
      City races.
";
        let parse = parse(source);
        assert_eq!(parse.errors(), []);
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  Ballot
    BallotKeyword \"ballot\"
    Identifier \"@franklin\"
    StringLiteral \"\\\"State of Franklin\\\"\"
    Newline \"\\n\"
    Block
      Indent \"\"
      StringField
        EligibilityKeyword \"eligibility\"
        StringLiteral \"\\\"Registered voters of Franklin\\\"\"
        Newline \"\\n\"
      Section
        SectionKeyword \"section\"
        StringLiteral \"\\\"Statewide Offices\\\"\"
        Newline \"\\n\"
        Block
          Indent \"\"
          MarkdownField
            NoteKeyword \"note\"
            Colon \":\"
            Newline \"\\n\"
            MarkdownBlock \"      Top of the ticket.\\n\"
          Dedent \"\"
      Ballot
        BallotKeyword \"ballot\"
        Identifier \"@ashford\"
        StringLiteral \"\\\"City of Ashford\\\"\"
        Newline \"\\n\"
        Block
          Indent \"\"
          MarkdownField
            NoteKeyword \"note\"
            Colon \":\"
            Newline \"\\n\"
            MarkdownBlock \"      City races.\\n\"
          Dedent \"\"
      Dedent \"\"
"
        );
    }

    #[test]
    fn ballots_nest_without_limit_and_roots_repeat() {
        let parse = parse(
            "ballot @a \"A\"\n  ballot @b \"B\"\n    ballot @c \"C\"\n      note:\n        deep\nballot @d \"D\"\n  note:\n    x\n",
        );
        assert_eq!(parse.errors(), []);
        let roots: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(roots, [SyntaxKind::Ballot, SyntaxKind::Ballot]);
        let mut depth = 0;
        let mut node = parse.syntax().first_child().unwrap();
        while let Some(inner) = node
            .last_child()
            .and_then(|b| b.children().find(|n| n.kind() == SyntaxKind::Ballot))
        {
            depth += 1;
            node = inner;
        }
        assert_eq!(depth, 2);
    }

    #[test]
    fn ballot_fields_and_items_interleave_but_fields_appear_once() {
        let parse = parse(
            "ballot @a \"A\"\n  section \"S1\"\n    note:\n      x\n  eligibility \"e\"\n  section \"S2\"\n    note:\n      y\n  eligibility \"again\"\n",
        );
        let block = parse.syntax().first_child().unwrap().last_child().unwrap();
        let kinds: Vec<SyntaxKind> = block.children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::Section,
                SyntaxKind::StringField,
                SyntaxKind::Section,
                SyntaxKind::ErrorNode
            ]
        );
        assert_eq!(parse.errors().len(), 1);
        assert!(matches!(
            &parse.errors()[0],
            SyntaxError::Parse(e) if e.found == Some(SyntaxKind::EligibilityKeyword)
                && e.expected == [SyntaxKind::NoteKeyword]
        ));
    }

    #[test]
    fn unknown_lines_in_a_section_name_note_contest_and_measure() {
        let source = "ballot @a \"A\"\n  section \"S\"\n    campaign @x \"X\"\n";
        let parse = parse(source);
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![
                    SyntaxKind::NoteKeyword,
                    SyntaxKind::ContestKeyword,
                    SyntaxKind::MeasureKeyword,
                ],
                found: Some(SyntaxKind::CampaignKeyword),
                range: span(source, "campaign @x \"X\"\n"),
            })]
        );
    }

    #[test]
    fn sections_do_not_nest_and_do_not_hold_ballots() {
        let parse = parse(
            "ballot @a \"A\"\n  section \"S\"\n    section \"Inner\"\n      note:\n        x\n    ballot @b \"B\"\n      note:\n        y\n    note:\n      ok\n",
        );
        let section = parse
            .syntax()
            .first_child()
            .unwrap()
            .last_child()
            .unwrap()
            .first_child()
            .unwrap();
        assert_eq!(section.kind(), SyntaxKind::Section);
        let kinds: Vec<SyntaxKind> = section
            .last_child()
            .unwrap()
            .children()
            .map(|n| n.kind())
            .collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::ErrorNode,
                SyntaxKind::ErrorNode,
                SyntaxKind::MarkdownField
            ]
        );
        assert_eq!(parse.errors().len(), 2);
        assert!(
            matches!(&parse.errors()[0], SyntaxError::Parse(e) if e.found == Some(SyntaxKind::SectionKeyword))
        );
        assert!(
            matches!(&parse.errors()[1], SyntaxError::Parse(e) if e.found == Some(SyntaxKind::BallotKeyword))
        );
    }

    #[test]
    fn ballots_and_sections_require_a_block() {
        let result = parse("ballot @a \"A\"\nballot @b \"B\"\n  note:\n    x\n");
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::Indent],
                found: Some(SyntaxKind::BallotKeyword),
                range: range(14, 14),
            })]
        );
        let result = parse("ballot @a \"A\"\n  section \"S\"\n  note:\n    x\n");
        assert_eq!(result.errors().len(), 1);
        assert!(
            matches!(&result.errors()[0], SyntaxError::Parse(e) if e.expected == [SyntaxKind::Indent])
        );
    }

    #[test]
    fn declarations_must_precede_ballots() {
        let parse = parse("ballot @a \"A\"\n  note:\n    x\nparty @d \"D\"\n");
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(kinds, [SyntaxKind::Ballot, SyntaxKind::ErrorNode]);
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::BallotKeyword],
                found: Some(SyntaxKind::PartyKeyword),
                range: range(28, 41),
            })]
        );
    }

    #[test]
    fn contest_header_and_fields_from_the_spec_example() {
        let source = in_ballot(
            "contest @mayor \"Mayor\" ranked max 5\n  subtitle \"Rank up to five candidates\"\n  tabulation \"Instant runoff\"\n  note:\n    Open seat.\n",
        );
        let parse = parse(&source);
        assert_eq!(parse.errors(), []);
        assert_eq!(
            dump(&first_contest(&parse)),
            "\
Contest
  ContestKeyword \"contest\"
  Identifier \"@mayor\"
  StringLiteral \"\\\"Mayor\\\"\"
  ContestKind
    RankedKeyword \"ranked\"
    MaxKeyword \"max\"
    IntegerLiteral \"5\"
  Newline \"\\n\"
  Block
    Indent \"\"
    StringField
      SubtitleKeyword \"subtitle\"
      StringLiteral \"\\\"Rank up to five candidates\\\"\"
      Newline \"\\n\"
    StringField
      TabulationKeyword \"tabulation\"
      StringLiteral \"\\\"Instant runoff\\\"\"
      Newline \"\\n\"
    MarkdownField
      NoteKeyword \"note\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"      Open seat.\\n\"
    Dedent \"\"
"
        );
    }

    #[test]
    fn every_contest_kind() {
        for (kind, expected) in [
            ("single", "    SingleKeyword \"single\"\n"),
            ("retention", "    RetentionKeyword \"retention\"\n"),
            (
                "multiple 3",
                "    MultipleKeyword \"multiple\"\n    IntegerLiteral \"3\"\n",
            ),
            ("ranked", "    RankedKeyword \"ranked\"\n"),
            (
                "ranked max 10",
                "    RankedKeyword \"ranked\"\n    MaxKeyword \"max\"\n    IntegerLiteral \"10\"\n",
            ),
        ] {
            let source = in_ballot(&format!("contest @c \"C\" {kind}\n  note:\n    x\n"));
            let parse = parse(&source);
            assert_eq!(parse.errors(), [], "{kind}");
            let contest = dump(&first_contest(&parse));
            let kind_node = contest
                .split("  ContestKind\n")
                .nth(1)
                .and_then(|rest| rest.split("  Newline").next())
                .expect("ContestKind in dump");
            assert_eq!(kind_node, expected, "{kind}");
        }
    }

    #[test]
    fn a_missing_or_unknown_contest_kind_is_one_error() {
        let source = in_ballot("contest @c \"C\"\n  note:\n    x\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![
                    SyntaxKind::SingleKeyword,
                    SyntaxKind::MultipleKeyword,
                    SyntaxKind::RankedKeyword,
                    SyntaxKind::RetentionKeyword,
                ],
                found: Some(SyntaxKind::Newline),
                range: TextRange::empty(span(&source, "\"C\"").end()),
            })]
        );

        let source = in_ballot("contest @c \"C\" yes no\n  note:\n    x\n");
        let result = parse(&source);
        assert_eq!(result.errors().len(), 1);
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.found == Some(SyntaxKind::YesKeyword)
                && e.range == span(&source, "yes no")
        ));
    }

    #[test]
    fn multiple_requires_a_seat_count() {
        let source = in_ballot("contest @c \"C\" multiple\n  note:\n    x\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::IntegerLiteral],
                found: Some(SyntaxKind::Newline),
                range: TextRange::empty(span(&source, "multiple").end()),
            })]
        );

        let source = in_ballot("contest @c \"C\" multiple \"3\"\n  note:\n    x\n");
        let result = parse(&source);
        assert_eq!(result.errors().len(), 1);
        let kind_node = first_contest(&result)
            .children()
            .find(|n| n.kind() == SyntaxKind::ContestKind)
            .unwrap();
        assert!(
            kind_node
                .children()
                .any(|n| n.kind() == SyntaxKind::ErrorNode)
        );
    }

    #[test]
    fn contest_block_fields_once_and_items_recovered_until_they_parse() {
        let source = in_ballot(
            "contest @c \"C\" single\n  subtitle \"a\"\n  campaign @x \"X\"\n  subtitle \"b\"\n  recommend\n    support @x\n",
        );
        let result = parse(&source);
        let block = first_contest(&result).last_child().unwrap();
        let kinds: Vec<SyntaxKind> = block.children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::StringField,
                SyntaxKind::Campaign,
                SyntaxKind::ErrorNode,
                SyntaxKind::ContestRecommend
            ]
        );
        assert_eq!(result.errors().len(), 1);
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.found == Some(SyntaxKind::SubtitleKeyword)
                && e.expected == [SyntaxKind::TabulationKeyword, SyntaxKind::NoteKeyword]
        ));
    }

    #[test]
    fn contests_live_in_ballots_and_sections_and_need_a_block() {
        let source = "ballot @b \"B\"\n  contest @a \"A\" single\n    note:\n      x\n  section \"S\"\n    contest @c \"C\" single\n      note:\n        y\n";
        let result = parse(source);
        assert_eq!(result.errors(), []);
        let contests = result
            .syntax()
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Contest)
            .count();
        assert_eq!(contests, 2);

        let source = in_ballot("contest @c \"C\" single\nnote:\n  x\n");
        let result = parse(&source);
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.expected == [SyntaxKind::Indent]
        ));
    }

    #[test]
    fn one_line_campaign_with_an_inline_candidate_from_the_spec_example() {
        let source = in_contest(
            "campaign incumbent @bsantos \"Beatriz Santos\"\n  designation \"Mayor of Ashford\"\n  website https://santosformayor.example\n  note:\n    Two-term incumbent.\n",
        );
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        assert_eq!(
            dump(&first(&result, SyntaxKind::Campaign)),
            "\
Campaign
  CampaignKeyword \"campaign\"
  Flag
    IncumbentKeyword \"incumbent\"
  Member
    Identifier \"@bsantos\"
    StringLiteral \"\\\"Beatriz Santos\\\"\"
  Newline \"\\n\"
  Block
    Indent \"\"
    StringField
      DesignationKeyword \"designation\"
      StringLiteral \"\\\"Mayor of Ashford\\\"\"
      Newline \"\\n\"
    UrlField
      WebsiteKeyword \"website\"
      UrlLiteral \"https://santosformayor.example\"
      Newline \"\\n\"
    MarkdownField
      NoteKeyword \"note\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"        Two-term incumbent.\\n\"
    Dedent \"\"
"
        );
    }

    #[test]
    fn block_form_campaign_with_a_ticket_from_the_spec_example() {
        let source = in_contest(
            "campaign incumbent\n  @jlee \"Jordan Lee\" as \"Governor\"\n    designation \"Governor of Franklin\"\n    party @dem\n  @mpatel \"Maya Patel\" as \"Lieutenant Governor\"\n    designation \"State Senator\"\n  website https://leepatel2026.example\n  note:\n    The ticket's note.\n",
        );
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        assert_eq!(
            dump(&first(&result, SyntaxKind::Campaign)),
            "\
Campaign
  CampaignKeyword \"campaign\"
  Flag
    IncumbentKeyword \"incumbent\"
  Newline \"\\n\"
  Block
    Indent \"\"
    Member
      Identifier \"@jlee\"
      StringLiteral \"\\\"Jordan Lee\\\"\"
      AsKeyword \"as\"
      StringLiteral \"\\\"Governor\\\"\"
      Newline \"\\n\"
      Block
        Indent \"\"
        StringField
          DesignationKeyword \"designation\"
          StringLiteral \"\\\"Governor of Franklin\\\"\"
          Newline \"\\n\"
        PartyReference
          PartyKeyword \"party\"
          Identifier \"@dem\"
          Newline \"\\n\"
        Dedent \"\"
    Member
      Identifier \"@mpatel\"
      StringLiteral \"\\\"Maya Patel\\\"\"
      AsKeyword \"as\"
      StringLiteral \"\\\"Lieutenant Governor\\\"\"
      Newline \"\\n\"
      Block
        Indent \"\"
        StringField
          DesignationKeyword \"designation\"
          StringLiteral \"\\\"State Senator\\\"\"
          Newline \"\\n\"
        Dedent \"\"
    UrlField
      WebsiteKeyword \"website\"
      UrlLiteral \"https://leepatel2026.example\"
      Newline \"\\n\"
    MarkdownField
      NoteKeyword \"note\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"        The ticket's note.\\n\"
    Dedent \"\"
"
        );
    }

    #[test]
    fn bare_references_flags_and_inline_parties() {
        let source = in_contest(
            "campaign @adam-schiff\ncampaign incumbent withdrawn @x \"X\"\ncampaign @g \"G\"\n  party @green \"Green\"\n    abbreviation \"G\"\n",
        );
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        let campaigns: Vec<_> = result
            .syntax()
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Campaign)
            .collect();
        assert_eq!(campaigns.len(), 3);
        let kinds = |n: &crate::SyntaxNode| n.children().map(|c| c.kind()).collect::<Vec<_>>();
        assert_eq!(kinds(&campaigns[0]), [SyntaxKind::Member]);
        assert_eq!(
            kinds(&campaigns[1]),
            [SyntaxKind::Flag, SyntaxKind::Flag, SyntaxKind::Member]
        );
        assert!(
            campaigns[2]
                .descendants()
                .any(|n| n.kind() == SyntaxKind::PartyReference)
        );
    }

    #[test]
    fn a_repeated_flag_is_an_error_node() {
        let source = in_contest("campaign incumbent incumbent @x \"X\"\n");
        let result = parse(&source);
        let second = span(&source, "incumbent incumbent").end() - TextSize::from(9);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::WithdrawnKeyword, SyntaxKind::Identifier],
                found: Some(SyntaxKind::IncumbentKeyword),
                range: TextRange::at(second, TextSize::from(9)),
            })]
        );
    }

    #[test]
    fn a_campaign_needs_a_member_or_a_block() {
        let source = in_contest("campaign\ncampaign @y \"Y\"\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::Indent],
                found: Some(SyntaxKind::CampaignKeyword),
                range: TextRange::empty(span(&source, "campaign @y").start()),
            })]
        );
    }

    #[test]
    fn a_role_needs_a_string() {
        let source = in_contest("campaign @x \"X\" as\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::StringLiteral],
                found: Some(SyntaxKind::Newline),
                range: TextRange::empty(span(&source, " as").end()),
            })]
        );
    }

    #[test]
    fn flags_after_the_member_are_extra_tokens() {
        let source = in_contest("campaign @x \"X\" incumbent\n");
        let result = parse(&source);
        assert_eq!(result.errors().len(), 1);
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.expected == [SyntaxKind::Newline]
                && e.range == span(&source, "incumbent")
        ));
    }

    #[test]
    fn unknown_lines_in_a_campaign_block_name_fields_and_members() {
        let source = in_contest("campaign @x \"X\"\n  subtitle \"no\"\n  website https://x.y\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![
                    SyntaxKind::WebsiteKeyword,
                    SyntaxKind::NoteKeyword,
                    SyntaxKind::DesignationKeyword,
                    SyntaxKind::PartyKeyword,
                    SyntaxKind::Identifier,
                ],
                found: Some(SyntaxKind::SubtitleKeyword),
                range: span(&source, "subtitle \"no\"\n"),
            })]
        );
    }

    #[test]
    fn a_reference_with_a_member_block_parses_and_is_left_to_validation() {
        let source = in_contest("campaign\n  @x\n    designation \"d\"\n");
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        let member = first(&result, SyntaxKind::Member);
        assert!(member.children().any(|n| n.kind() == SyntaxKind::Block));
    }

    #[test]
    fn contest_recommendation_from_the_spec_example() {
        let source = in_contest(
            "recommend\n  support @aokafor\n  support @bsantos @dwright\n  oppose @cfox\n  justification:\n    Okafor is our first choice.\n  note:\n    Do not rank Fox.\n",
        );
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        assert_eq!(
            dump(&first(&result, SyntaxKind::ContestRecommend)),
            "\
ContestRecommend
  RecommendKeyword \"recommend\"
  Newline \"\\n\"
  Block
    Indent \"\"
    Support
      SupportKeyword \"support\"
      Identifier \"@aokafor\"
      Newline \"\\n\"
    Support
      SupportKeyword \"support\"
      Identifier \"@bsantos\"
      Identifier \"@dwright\"
      Newline \"\\n\"
    Oppose
      OpposeKeyword \"oppose\"
      Identifier \"@cfox\"
      Newline \"\\n\"
    MarkdownField
      JustificationKeyword \"justification\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"        Okafor is our first choice.\\n\"
    MarkdownField
      NoteKeyword \"note\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"        Do not rank Fox.\\n\"
    Dedent \"\"
"
        );
    }

    #[test]
    fn no_position_with_and_without_a_block() {
        let source = in_contest("recommend no-position\n  justification:\n    Split board.\n");
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        assert_eq!(
            dump(&first(&result, SyntaxKind::ContestRecommend)),
            "\
ContestRecommend
  RecommendKeyword \"recommend\"
  NoPositionKeyword \"no-position\"
  Newline \"\\n\"
  Block
    Indent \"\"
    MarkdownField
      JustificationKeyword \"justification\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"        Split board.\\n\"
    Dedent \"\"
"
        );

        let source = in_contest("recommend no-position\n");
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        assert_eq!(
            first(&result, SyntaxKind::ContestRecommend)
                .children()
                .count(),
            0
        );
    }

    #[test]
    fn a_recommendation_with_no_stance_must_say_no_position() {
        let source = in_contest("recommend\n  justification:\n    Why.\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![
                    SyntaxKind::SupportKeyword,
                    SyntaxKind::OpposeKeyword,
                    SyntaxKind::NoPositionKeyword,
                ],
                found: Some(SyntaxKind::RecommendKeyword),
                range: span(&source, "recommend"),
            })]
        );
    }

    #[test]
    fn a_bare_recommend_needs_a_block() {
        let source = in_contest("recommend\ncampaign @x \"X\"\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::Indent],
                found: Some(SyntaxKind::CampaignKeyword),
                range: TextRange::empty(span(&source, "campaign @x").start()),
            })]
        );
    }

    #[test]
    fn support_and_oppose_need_identifiers_and_nothing_else() {
        let source = in_contest("recommend\n  support\n  oppose @a \"x\"\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [
                SyntaxError::Parse(ParseError {
                    expected: vec![SyntaxKind::Identifier],
                    found: Some(SyntaxKind::Newline),
                    range: TextRange::empty(span(&source, "support").end()),
                }),
                SyntaxError::Parse(ParseError {
                    expected: vec![SyntaxKind::Identifier, SyntaxKind::Newline],
                    found: Some(SyntaxKind::StringLiteral),
                    range: span(&source, "\"x\""),
                }),
            ]
        );
        let kinds: Vec<SyntaxKind> = first(&result, SyntaxKind::ContestRecommend)
            .last_child()
            .unwrap()
            .children()
            .map(|n| n.kind())
            .collect();
        assert_eq!(kinds, [SyntaxKind::Support, SyntaxKind::Oppose]);
    }

    #[test]
    fn a_measure_style_position_is_rejected_in_a_contest() {
        let source = in_contest("recommend yes\n  justification:\n    Why.\n");
        let result = parse(&source);
        assert_eq!(result.errors().len(), 1);
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.expected == [SyntaxKind::NoPositionKeyword, SyntaxKind::Newline]
                && e.range == span(&source, "yes")
        ));
        assert!(
            first(&result, SyntaxKind::ContestRecommend)
                .children()
                .any(|n| n.kind() == SyntaxKind::Block)
        );
    }

    #[test]
    fn stances_are_not_allowed_under_no_position() {
        let source = in_contest("recommend no-position\n  support @a\n  note:\n    x\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::JustificationKeyword, SyntaxKind::NoteKeyword],
                found: Some(SyntaxKind::SupportKeyword),
                range: span(&source, "support @a\n"),
            })]
        );
    }

    #[test]
    fn repeated_oppose_and_recommend_are_left_to_validation() {
        let source = in_contest("recommend\n  oppose @a\n  oppose @b\nrecommend\n  support @c\n");
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        let recommends = result
            .syntax()
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::ContestRecommend)
            .count();
        assert_eq!(recommends, 2);
    }

    #[test]
    fn measure_from_the_spec_example() {
        let source = in_ballot(
            "measure @prop1 \"Proposition 1\"\n  subtitle \"Affordable Housing Bond\"\n  description:\n    Authorizes $2 billion in bonds.\n  if_yes:\n    Bonds may be sold.\n  if_no:\n    They may not.\n  placement \"Legislative referral\"\n  tabulation \"Simple majority\"\n  fiscal_impact \"About $140 million a year\"\n  supporters \"Franklin Housing Coalition\", \"Franklin Labor Federation\"\n  opponents none\n\n  recommend yes\n    justification:\n      The only funded plan.\n",
        );
        let result = parse(&source);
        assert_eq!(result.errors(), []);
        assert_eq!(
            dump(&first(&result, SyntaxKind::Measure)),
            "\
Measure
  MeasureKeyword \"measure\"
  Identifier \"@prop1\"
  StringLiteral \"\\\"Proposition 1\\\"\"
  Newline \"\\n\"
  Block
    Indent \"\"
    StringField
      SubtitleKeyword \"subtitle\"
      StringLiteral \"\\\"Affordable Housing Bond\\\"\"
      Newline \"\\n\"
    MarkdownField
      DescriptionKeyword \"description\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"      Authorizes $2 billion in bonds.\\n\"
    MarkdownField
      IfYesKeyword \"if_yes\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"      Bonds may be sold.\\n\"
    MarkdownField
      IfNoKeyword \"if_no\"
      Colon \":\"
      Newline \"\\n\"
      MarkdownBlock \"      They may not.\\n\"
    StringField
      PlacementKeyword \"placement\"
      StringLiteral \"\\\"Legislative referral\\\"\"
      Newline \"\\n\"
    StringField
      TabulationKeyword \"tabulation\"
      StringLiteral \"\\\"Simple majority\\\"\"
      Newline \"\\n\"
    StringField
      FiscalImpactKeyword \"fiscal_impact\"
      StringLiteral \"\\\"About $140 million a year\\\"\"
      Newline \"\\n\"
    StringListField
      SupportersKeyword \"supporters\"
      StringLiteral \"\\\"Franklin Housing Coalition\\\"\"
      Comma \",\"
      StringLiteral \"\\\"Franklin Labor Federation\\\"\"
      Newline \"\\n\"
    StringListField
      OpponentsKeyword \"opponents\"
      NoneValueKeyword \"none\"
      Newline \"\\n\"
    MeasureRecommend
      RecommendKeyword \"recommend\"
      YesKeyword \"yes\"
      Newline \"\\n\"
      Block
        Indent \"\"
        MarkdownField
          JustificationKeyword \"justification\"
          Colon \":\"
          Newline \"\\n\"
          MarkdownBlock \"        The only funded plan.\\n\"
        Dedent \"\"
    Dedent \"\"
"
        );
    }

    #[test]
    fn string_lists() {
        let source = in_ballot(
            "measure @m \"M\"\n  supporters \"a\"\n  opponents \"a\", \"b\", \"c\"\n  recommend no\n",
        );
        let result = parse(&source);
        assert_eq!(result.errors(), []);

        let source =
            in_ballot("measure @m \"M\"\n  supporters\n  opponents \"a\",\n  recommend no\n");
        let result = parse(&source);
        assert_eq!(
            result.errors(),
            [
                SyntaxError::Parse(ParseError {
                    expected: vec![SyntaxKind::StringLiteral, SyntaxKind::NoneValueKeyword],
                    found: Some(SyntaxKind::Newline),
                    range: TextRange::empty(span(&source, "supporters").end()),
                }),
                SyntaxError::Parse(ParseError {
                    expected: vec![SyntaxKind::StringLiteral],
                    found: Some(SyntaxKind::Newline),
                    range: TextRange::empty(span(&source, "\"a\",").end()),
                }),
            ]
        );

        let source = in_ballot("measure @m \"M\"\n  supporters \"a\" \"b\"\n  recommend no\n");
        let result = parse(&source);
        assert_eq!(result.errors().len(), 1);
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.expected == [SyntaxKind::Comma, SyntaxKind::Newline]
                && e.range == span(&source, "\"b\"")
        ));
    }

    #[test]
    fn measure_positions() {
        for position in ["yes", "no", "no-position"] {
            let source = in_ballot(&format!("measure @m \"M\"\n  recommend {position}\n"));
            let result = parse(&source);
            assert_eq!(result.errors(), [], "{position}");
        }

        let source = in_ballot("measure @m \"M\"\n  recommend\n    support @a\n");
        let result = parse(&source);
        assert_eq!(result.errors().len(), 2);
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.expected == MEASURE_POSITIONS_FOR_TEST
                && e.range == TextRange::empty(span(&source, "recommend").end())
        ));
        assert!(matches!(
            &result.errors()[1],
            SyntaxError::Parse(e) if e.found == Some(SyntaxKind::SupportKeyword)
                && e.expected == [SyntaxKind::JustificationKeyword, SyntaxKind::NoteKeyword]
        ));

        let source = in_ballot("measure @m \"M\"\n  recommend maybe\n");
        let result = parse(&source);
        assert_eq!(result.errors().len(), 2);
        assert!(matches!(&result.errors()[0], SyntaxError::Lex(_)));
        assert!(matches!(
            &result.errors()[1],
            SyntaxError::Parse(e) if e.expected == MEASURE_POSITIONS_FOR_TEST && e.range == span(&source, "maybe")
        ));
    }

    const MEASURE_POSITIONS_FOR_TEST: [SyntaxKind; 3] = [
        SyntaxKind::YesKeyword,
        SyntaxKind::NoKeyword,
        SyntaxKind::NoPositionKeyword,
    ];

    #[test]
    fn measures_live_in_ballots_and_sections_with_fields_once() {
        let source = "ballot @b \"B\"\n  measure @a \"A\"\n    recommend yes\n  section \"S\"\n    measure @c \"C\"\n      subtitle \"x\"\n      subtitle \"y\"\n      recommend no\n";
        let result = parse(source);
        let measures = result
            .syntax()
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Measure)
            .count();
        assert_eq!(measures, 2);
        assert_eq!(result.errors().len(), 1);
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.found == Some(SyntaxKind::SubtitleKeyword)
                && e.expected.first() == Some(&SyntaxKind::DescriptionKeyword)
        ));
    }

    #[test]
    fn the_embedded_sample_parses_clean() {
        let result = parse(SAMPLE);
        assert_eq!(result.errors(), []);
    }
}
