use super::{
    AstNode, Ballot, DateField, Identifier, MarkdownField, StringField, UrlField, ast_node, block,
    child, children, field, identifier, string, token,
};
use crate::{Keyword, SpecVersion, SyntaxKind, SyntaxNode};

ast_node! {
    /// A whole guide: the root of the tree.
    Document
}

ast_node! {
    /// The `oval` version marker that opens a document.
    VersionMarker
}

ast_node! {
    /// A `party` declaration.
    Party
}

ast_node! {
    /// A `candidate` declaration.
    Candidate
}

ast_node! {
    /// A `party` line on a candidate or campaign: a reference, or an inline declaration.
    PartyReference
}

impl Document {
    /// The `oval` version marker.
    pub fn version(&self) -> Option<VersionMarker> {
        child(&self.0)
    }

    /// The `guide` field.
    pub fn guide(&self) -> Option<StringField> {
        top_level_field(&self.0, Keyword::Guide)
    }

    /// The `election` field.
    pub fn election(&self) -> Option<StringField> {
        top_level_field(&self.0, Keyword::Election)
    }

    /// The `date` field.
    pub fn date(&self) -> Option<DateField> {
        top_level_field(&self.0, Keyword::Date)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        top_level_field(&self.0, Keyword::Note)
    }

    /// Top-level party declarations, in source order.
    pub fn parties(&self) -> impl Iterator<Item = Party> + use<> {
        children(&self.0)
    }

    /// Top-level candidate declarations, in source order.
    pub fn candidates(&self) -> impl Iterator<Item = Candidate> + use<> {
        children(&self.0)
    }

    /// Root ballots, in source order.
    pub fn ballots(&self) -> impl Iterator<Item = Ballot> + use<> {
        children(&self.0)
    }
}

impl VersionMarker {
    /// The declared specification version.
    pub fn value(&self) -> Option<SpecVersion> {
        let token = token(&self.0, SyntaxKind::VersionLiteral)?;
        let (major, minor) = token.text().split_once('.')?;
        Some(SpecVersion {
            major: major.parse().ok()?,
            minor: minor.parse().ok()?,
        })
    }
}

impl Party {
    /// The declared identifier.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The party's name as printed on the ballot.
    pub fn name(&self) -> Option<String> {
        string(&self.0)
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
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The candidate's name as printed on the ballot.
    pub fn name(&self) -> Option<String> {
        string(&self.0)
    }

    /// The `designation` field.
    pub fn designation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Designation)
    }

    /// The `party` line.
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
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The party's name, present only when this line declares the party.
    pub fn name(&self) -> Option<String> {
        string(&self.0)
    }

    /// Whether this line declares the party rather than referring to one declared elsewhere.
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
