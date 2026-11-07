//! A contest's recommendation, which is tiers of support and a line of opposition.
//!
//! A [`ContestRecommend`] is either `recommend no-position` or a block of [`Support`] tiers, at
//! most one [`Oppose`] line, and the fields `justification` and `note` (Section 2.15). Every
//! reference in a tier or an oppose line is a candidate identifier that names a campaign of the
//! enclosing contest by one of its members.

use super::{Identifier, MarkdownField, ast_node, block_children, field, has_token, identifiers};
use crate::{Keyword, SyntaxKind};

ast_node! {
    /// A contest's `recommend`.
    ///
    /// `recommend` with a block takes a position; `recommend no-position` is an explicit statement
    /// that the author takes none and may carry only `justification` and `note`. A block with
    /// neither `support` nor `oppose` is the syntax error `empty-recommendation`.
    ContestRecommend
}

ast_node! {
    /// A tier of campaigns the author considers interchangeable, named by candidate, written as
    /// `support`.
    ///
    /// Several tiers are several `support` lines, in order from most to least preferred.
    Support
}

ast_node! {
    /// The campaigns the author advises against, named by candidate, written as `oppose`.
    ///
    /// The line is unordered. A valid recommendation has at most one (`repeated-oppose`).
    Oppose
}

impl ContestRecommend {
    /// Whether this is `recommend no-position`.
    pub fn is_no_position(&self) -> bool {
        has_token(&self.0, SyntaxKind::NoPositionKeyword)
    }

    /// The support tiers, most preferred first.
    ///
    /// Empty for `recommend no-position` and for a recommendation with only an `oppose` line.
    pub fn support(&self) -> impl Iterator<Item = Support> + use<> {
        block_children(&self.0)
    }

    /// Every `oppose` line, in source order.
    ///
    /// A valid recommendation has at most one; this returns them all so that validation can
    /// report a second one where it is written.
    pub fn oppose(&self) -> impl Iterator<Item = Oppose> + use<> {
        block_children(&self.0)
    }

    /// Why the author recommends this, from the `justification` field.
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
    ///
    /// Each names the campaign of the enclosing contest that the candidate belongs to. An `@`
    /// with no word after it is skipped.
    pub fn candidates(&self) -> impl Iterator<Item = Identifier> + use<> {
        identifiers(&self.0)
    }
}

impl Oppose {
    /// The candidates named, in source order.
    ///
    /// Each names the campaign of the enclosing contest that the candidate belongs to. An `@`
    /// with no word after it is skipped.
    pub fn candidates(&self) -> impl Iterator<Item = Identifier> + use<> {
        identifiers(&self.0)
    }
}
