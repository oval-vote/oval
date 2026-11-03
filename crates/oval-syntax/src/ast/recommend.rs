use super::{Identifier, MarkdownField, ast_node, block_children, field, has_token, tokens};
use crate::{Keyword, SyntaxKind};

ast_node! {
    /// A contest's `recommend`.
    ContestRecommend
}

ast_node! {
    /// One `support` tier: campaigns the author considers interchangeable, named by candidate.
    Support
}

ast_node! {
    /// An `oppose` line: campaigns the author advises against, named by candidate.
    Oppose
}

impl ContestRecommend {
    /// Whether this is `recommend no-position`.
    pub fn is_no_position(&self) -> bool {
        has_token(&self.0, SyntaxKind::NoPositionKeyword)
    }

    /// The support tiers, most preferred first.
    pub fn support(&self) -> impl Iterator<Item = Support> + use<> {
        block_children(&self.0)
    }

    /// Every `oppose` line. A valid recommendation has at most one.
    pub fn oppose(&self) -> impl Iterator<Item = Oppose> + use<> {
        block_children(&self.0)
    }

    /// The `justification` field.
    pub fn justification(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Justification)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }
}

impl Support {
    /// The candidates named, in source order.
    pub fn candidates(&self) -> impl Iterator<Item = Identifier> + use<> {
        identifiers(&self.0)
    }
}

impl Oppose {
    /// The candidates named, in source order.
    pub fn candidates(&self) -> impl Iterator<Item = Identifier> + use<> {
        identifiers(&self.0)
    }
}

fn identifiers(node: &crate::SyntaxNode) -> impl Iterator<Item = Identifier> + use<> {
    tokens(node)
        .filter(|t| t.kind() == SyntaxKind::Identifier)
        .map(Identifier)
}
