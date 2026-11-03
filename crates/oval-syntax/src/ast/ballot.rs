use super::{
    AstNode, Contest, Identifier, MarkdownField, Measure, StringField, ast_node, block, field,
    identifier, string,
};
use crate::{Keyword, SyntaxNode};

ast_node! {
    /// A `ballot`: the questions, sections, and sub-ballots one electorate sees.
    Ballot
}

ast_node! {
    /// A `section`: a titled group of questions within a ballot.
    Section
}

/// One entry in a ballot's block, in ballot order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BallotItem {
    /// A titled group of questions.
    Section(Section),
    /// A contest.
    Contest(Contest),
    /// A measure.
    Measure(Measure),
    /// A sub-ballot.
    Ballot(Ballot),
}

/// One entry in a section's block, in ballot order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectionItem {
    /// A contest.
    Contest(Contest),
    /// A measure.
    Measure(Measure),
}

impl Ballot {
    /// The declared identifier.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The jurisdiction or grouping this ballot covers.
    pub fn title(&self) -> Option<String> {
        string(&self.0)
    }

    /// The `eligibility` field.
    pub fn eligibility(&self) -> Option<StringField> {
        field(&self.0, Keyword::Eligibility)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// The items of the block, in ballot order.
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
    /// The section heading.
    pub fn title(&self) -> Option<String> {
        string(&self.0)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// The questions of the block, in ballot order.
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
