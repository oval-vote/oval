use oval_syntax::TextRange;
use oval_syntax::ast::{
    AstNode, Ballot, BallotItem, Contest, Document, Measure, Section, SectionItem,
};
use serde::Serialize;

use crate::symbols::serialize_range;

/// The kind of construct an outline item stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutlineKind {
    /// A ballot, root or nested.
    Ballot,
    /// A section.
    Section,
    /// A contest.
    Contest,
    /// A measure.
    Measure,
}

/// One construct in the outline of a document, with the constructs nested in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutlineItem {
    /// The kind of construct.
    pub kind: OutlineKind,
    /// The title as written, or `None` when the line ended before it.
    pub title: Option<String>,
    /// The declared name without its `@`, or `None` for a section or when the parser found none.
    pub name: Option<String>,
    /// The range of the whole construct, its block included.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// The range of the construct's header line, which an editor reveals when the item is chosen.
    #[serde(serialize_with = "serialize_range")]
    pub selection: TextRange,
    /// The constructs nested in this one, in ballot order.
    pub children: Vec<OutlineItem>,
}

/// The structure of `document` as nested ballots, sections, contests, and measures.
///
/// Root ballots come first, in document order. Parties and candidates are not in the outline,
/// since they are declarations rather than places on the ballot; [`Symbols`](crate::Symbols)
/// lists them.
pub fn outline(document: &Document) -> Vec<OutlineItem> {
    document
        .ballots()
        .map(|ballot| ballot_item(&ballot))
        .collect()
}

fn ballot_item(ballot: &Ballot) -> OutlineItem {
    OutlineItem {
        kind: OutlineKind::Ballot,
        title: ballot.title(),
        name: ballot.id().map(|id| id.name()),
        range: ballot.range(),
        selection: ballot.header_line_range(),
        children: ballot
            .items()
            .map(|item| match item {
                BallotItem::Ballot(sub) => ballot_item(&sub),
                BallotItem::Section(section) => section_item(&section),
                BallotItem::Contest(contest) => contest_item(&contest),
                BallotItem::Measure(measure) => measure_item(&measure),
            })
            .collect(),
    }
}

fn section_item(section: &Section) -> OutlineItem {
    OutlineItem {
        kind: OutlineKind::Section,
        title: section.title(),
        name: None,
        range: section.range(),
        selection: section.header_line_range(),
        children: section
            .items()
            .map(|item| match item {
                SectionItem::Contest(contest) => contest_item(&contest),
                SectionItem::Measure(measure) => measure_item(&measure),
            })
            .collect(),
    }
}

fn contest_item(contest: &Contest) -> OutlineItem {
    OutlineItem {
        kind: OutlineKind::Contest,
        title: contest.title(),
        name: contest.id().map(|id| id.name()),
        range: contest.range(),
        selection: contest.header_line_range(),
        children: Vec::new(),
    }
}

fn measure_item(measure: &Measure) -> OutlineItem {
    OutlineItem {
        kind: OutlineKind::Measure,
        title: measure.title(),
        name: measure.id().map(|id| id.name()),
        range: measure.range(),
        selection: measure.header_line_range(),
        children: Vec::new(),
    }
}
