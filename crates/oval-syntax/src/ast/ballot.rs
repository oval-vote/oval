//! Ballots and sections, the tree that places every question.
//!
//! A [`Ballot`] is what one electorate sees. Its items are contests, measures, sections, and
//! sub-ballots, in ballot order; a sub-ballot is a narrower electorate whose voters also see
//! everything in the enclosing ballot (Section 2.11). A [`Section`] is a titled group of questions
//! within a ballot and nests nothing else (Section 2.12).

use super::{
    AstNode, Contest, Identifier, MarkdownField, Measure, StringField, ast_node, block, field,
    identifier, string, string_range,
};
use crate::TextRange;
use crate::{Keyword, SyntaxNode};

ast_node! {
    /// The questions, sections, and sub-ballots one electorate sees, written as `ballot`.
    ///
    /// Root ballots are children of the [`Document`](super::Document); every other ballot is an
    /// item of its parent. A ballot requires a block (`missing-block`), and a valid ballot has at
    /// least one item (`empty-ballot`).
    Ballot
}

ast_node! {
    /// A titled group of questions within a ballot, written as `section`.
    ///
    /// A section matches a heading on the printed ballot. It has no identifier and no eligibility
    /// meaning, and a valid section has at least one question (`empty-section`).
    Section
}

/// One entry in a ballot's block, in ballot order.
///
/// Fields of the ballot, such as `eligibility` and `note`, are not items; [`Ballot::items`] skips
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BallotItem {
    /// A titled group of questions, written as `section`.
    Section(Section),
    /// An office chosen among campaigns, written as `contest`.
    Contest(Contest),
    /// A yes-or-no question, written as `measure`.
    Measure(Measure),
    /// A sub-ballot, which is a narrower electorate within this one.
    Ballot(Ballot),
}

/// One entry in a section's block, in ballot order.
///
/// Sections contain only questions, so there are two cases.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectionItem {
    /// An office chosen among campaigns, written as `contest`.
    Contest(Contest),
    /// A yes-or-no question, written as `measure`.
    Measure(Measure),
}

impl Ballot {
    /// The declared identifier.
    ///
    /// `None` when the parser recovered from a line with no identifier or an `@` with no word.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The jurisdiction or grouping this ballot covers, as printed.
    ///
    /// `None` when the line ended before the title.
    pub fn title(&self) -> Option<String> {
        string(&self.0)
    }

    /// The range of the ballot's title, quotes included.
    ///
    /// `None` when the line ended before the title.
    pub fn title_range(&self) -> Option<TextRange> {
        string_range(&self.0)
    }

    /// Who votes on this ballot, from the `eligibility` field.
    pub fn eligibility(&self) -> Option<StringField> {
        field(&self.0, Keyword::Eligibility)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// The items of the block, in ballot order.
    ///
    /// Empty when the ballot has no block or its block has only fields.
    pub fn items(&self) -> impl Iterator<Item = BallotItem> + use<> {
        block_nodes(&self.0).filter_map(|node| {
            Section::cast(node.clone())
                .map(BallotItem::Section)
                .or_else(|| Contest::cast(node.clone()).map(BallotItem::Contest))
                .or_else(|| Measure::cast(node.clone()).map(BallotItem::Measure))
                .or_else(|| Ballot::cast(node).map(BallotItem::Ballot))
        })
    }
}

impl Section {
    /// The section heading, as printed on the ballot.
    ///
    /// `None` when the line ended before the title.
    pub fn title(&self) -> Option<String> {
        string(&self.0)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// The questions of the block, in ballot order.
    ///
    /// Empty when the section has no block or its block has only a `note`.
    pub fn items(&self) -> impl Iterator<Item = SectionItem> + use<> {
        block_nodes(&self.0).filter_map(|node| {
            Contest::cast(node.clone())
                .map(SectionItem::Contest)
                .or_else(|| Measure::cast(node).map(SectionItem::Measure))
        })
    }
}

fn block_nodes(node: &SyntaxNode) -> impl Iterator<Item = SyntaxNode> + use<> {
    block(node).into_iter().flat_map(|b| b.children())
}
