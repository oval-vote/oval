//! The root of the tree and the declarations written at the top level.
//!
//! A [`Document`] is the guide of Section 2.8. It comprises the guide fields, the top-level
//! [`Party`] and [`Candidate`] declarations, and the root ballots. [`PartyReference`] is the
//! `party` line that a candidate or campaign writes, which either refers to a party declared
//! elsewhere or declares one inline.

use super::{
    AstNode, Ballot, DateField, Identifier, MarkdownField, StringField, UrlField, ast_node, block,
    child, children, field, identifier, string, string_range, token,
};
use crate::TextRange;
use crate::{Keyword, SyntaxKind, SyntaxNode};

ast_node! {
    /// A whole guide, the root of the tree.
    ///
    /// The root node covers the entire source. Its children are the version marker, the guide
    /// fields, the top-level declarations, and the root ballots, in the order Section 2.5 gives;
    /// an entry out of that order is still a child, and the parser reports `misplaced-entry`.
    ///
    /// # Examples
    ///
    /// ```
    /// use oval_syntax::ast::{AstNode, Document};
    /// use oval_syntax::parser::parse;
    ///
    /// let result = parse("oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\n").unwrap();
    /// let document = Document::cast(result.syntax()).unwrap();
    /// assert_eq!(document.guide().and_then(|f| f.value()).as_deref(), Some("G"));
    /// assert_eq!(document.date().and_then(|f| f.value()).map(|d| d.year), Some(2026));
    /// assert_eq!(document.ballots().count(), 0);
    /// ```
    Document
}

ast_node! {
    /// The `oval` version marker that opens a document.
    ///
    /// The marker names the major version of the language the document is written in. Whether
    /// this crate reads that version is the validation rule `unsupported-version`; see
    /// [`SUPPORTED_LANGUAGE_MAJOR`](crate::SUPPORTED_LANGUAGE_MAJOR).
    VersionMarker
}

ast_node! {
    /// A `party` declaration at the top level of the document.
    ///
    /// A party declared inline on a candidate or campaign is a [`PartyReference`] instead.
    Party
}

ast_node! {
    /// A `candidate` declaration at the top level of the document.
    ///
    /// A candidate declared inline in a campaign is a [`Member`](super::Member) instead.
    Candidate
}

ast_node! {
    /// A `party` line on a candidate or campaign, either a reference or an inline declaration.
    ///
    /// `party @dem` refers to a party declared elsewhere. `party @green "Green"` declares the party
    /// here, and may carry the same block of fields as a top-level [`Party`].
    /// [`is_declaration`](Self::is_declaration) tells the two apart.
    PartyReference
}

impl Document {
    /// The `oval` version marker.
    ///
    /// `None` when the document has no marker, which validation reports as `unsupported-version`.
    pub fn version(&self) -> Option<VersionMarker> {
        child(&self.0)
    }

    /// The name of the guide, from the `guide` field.
    ///
    /// `None` when the field is absent, which validation reports as `missing-guide-field`.
    pub fn guide(&self) -> Option<StringField> {
        top_level_field(&self.0, Keyword::Guide)
    }

    /// The name of the election, from the `election` field.
    ///
    /// `None` when the field is absent, which validation reports as `missing-guide-field`.
    pub fn election(&self) -> Option<StringField> {
        top_level_field(&self.0, Keyword::Election)
    }

    /// The date of the election, from the `date` field.
    ///
    /// `None` when the field is absent, which validation reports as `missing-guide-field`.
    pub fn date(&self) -> Option<DateField> {
        top_level_field(&self.0, Keyword::Date)
    }

    /// The `note` field, which is optional.
    pub fn note(&self) -> Option<MarkdownField> {
        top_level_field(&self.0, Keyword::Note)
    }

    /// The top-level party declarations, in source order.
    ///
    /// Parties declared inline are not included; see [`PartyReference`].
    pub fn parties(&self) -> impl Iterator<Item = Party> + use<> {
        children(&self.0)
    }

    /// The top-level candidate declarations, in source order.
    ///
    /// Candidates declared inline in a campaign are not included; see [`Member`](super::Member).
    pub fn candidates(&self) -> impl Iterator<Item = Candidate> + use<> {
        children(&self.0)
    }

    /// The root ballots, in source order.
    ///
    /// Sub-ballots are items of their enclosing ballot; see [`Ballot::items`]. A valid document
    /// has at least one root ballot (`empty-document`).
    pub fn ballots(&self) -> impl Iterator<Item = Ballot> + use<> {
        children(&self.0)
    }
}

impl VersionMarker {
    /// The declared major version of the language.
    ///
    /// Leading zeros are insignificant, so `oval 01` is version 1. `None` when the line ended
    /// before the integer or the integer does not fit in a `u32`.
    pub fn value(&self) -> Option<u32> {
        token(&self.0, SyntaxKind::IntegerLiteral)?
            .text()
            .parse()
            .ok()
    }
}

impl Party {
    /// The declared identifier.
    ///
    /// `None` when the parser recovered from a line with no identifier or an `@` with no word.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The party's name as printed on the ballot.
    ///
    /// `None` when the line ended before the name.
    pub fn name(&self) -> Option<String> {
        string(&self.0)
    }

    /// The range of the party's name, quotes included.
    ///
    /// `None` when the line ended before the name.
    pub fn name_range(&self) -> Option<TextRange> {
        string_range(&self.0)
    }

    /// The `abbreviation` field.
    pub fn abbreviation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Abbreviation)
    }

    /// The `website` field.
    pub fn website(&self) -> Option<UrlField> {
        field(&self.0, Keyword::Website)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }
}

impl Candidate {
    /// The declared identifier.
    ///
    /// `None` when the parser recovered from a line with no identifier or an `@` with no word.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The candidate's name as printed on the ballot.
    ///
    /// `None` when the line ended before the name.
    pub fn name(&self) -> Option<String> {
        string(&self.0)
    }

    /// The ballot designation, such as an occupation or current office, from the `designation`
    /// field.
    pub fn designation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Designation)
    }

    /// The `party` line, whether a reference or an inline declaration.
    pub fn party(&self) -> Option<PartyReference> {
        block(&self.0).and_then(|b| child(&b))
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }
}

impl PartyReference {
    /// The identifier referred to or declared.
    ///
    /// `None` when the parser recovered from a line with no identifier or an `@` with no word.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The party's name, present only when this line declares the party.
    pub fn name(&self) -> Option<String> {
        string(&self.0)
    }

    /// The range of the party's name, quotes included, when this line declares the party.
    pub fn name_range(&self) -> Option<TextRange> {
        string_range(&self.0)
    }

    /// Whether this line declares the party rather than referring to one declared elsewhere.
    ///
    /// A declaration is an identifier followed by a string (Section 2.7). The string decides;
    /// whether the identifier is also declared elsewhere is the validation rule
    /// `duplicate-declaration`.
    pub fn is_declaration(&self) -> bool {
        token(&self.0, SyntaxKind::StringLiteral).is_some()
    }

    /// The `abbreviation` field of an inline declaration.
    pub fn abbreviation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Abbreviation)
    }

    /// The `website` field of an inline declaration.
    pub fn website(&self) -> Option<UrlField> {
        field(&self.0, Keyword::Website)
    }

    /// The `note` field of an inline declaration.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }
}

fn top_level_field<N: AstNode>(node: &SyntaxNode, which: Keyword) -> Option<N> {
    node.children()
        .filter_map(N::cast)
        .find(|f| super::keyword(f.syntax()) == Some(which))
}
