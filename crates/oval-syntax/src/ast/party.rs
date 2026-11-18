use crate::Keyword;
use crate::ast::{
    AstNode, Identifier, MarkdownField, StringField, StringLiteral, UrlField, ast_node, support,
};

ast_node! {
    /// A party declared at the top level, such as `party @dem "Democratic"`, with the fields in the
    /// block below it.
    ///
    /// ```
    /// let text = "party @dem \"Democratic\"\n  abbreviation \"D\"\n";
    /// let result = oval_syntax::parse(text)?;
    /// let party = result.document().parties().next().expect("the document has a party");
    /// let name = party.name().expect("the party has a name");
    /// assert_eq!(name.value(), "Democratic");
    /// assert!(party.abbreviation().is_some());
    /// assert!(party.website().is_none());
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Party
}

impl Party {
    /// Returns the party's identifier, or `None` if the line has none.
    pub fn identifier(&self) -> Option<Identifier> {
        support::child(self.syntax())
    }

    /// Returns the party's name, or `None` if the line ends without one.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns the party's `abbreviation` field, or `None` if it has none.
    pub fn abbreviation(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Abbreviation)
    }

    /// Returns the party's `website` field, or `None` if it has none.
    pub fn website(&self) -> Option<UrlField> {
        support::block_field(self.syntax(), Keyword::Website)
    }

    /// Returns the party's `note` field, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }
}

#[cfg(test)]
mod tests {
    use super::Party;
    use crate::ast::{AstNode, AstToken};
    use crate::{SyntaxKind, parse};

    fn party(text: &str) -> Party {
        parse(text)
            .unwrap()
            .document()
            .parties()
            .next()
            .expect("the document has a party")
    }

    #[test]
    fn a_party_has_its_identifier_name_and_fields() {
        let party = party(
            "party @dem \"Democratic\"\n  abbreviation \"D\"\n  website https://dems.example\n  note:\n    Founded in 1828.\n",
        );
        let name = party.identifier().and_then(|id| id.name());
        assert_eq!(
            name.map(|name| name.text().to_owned()).as_deref(),
            Some("dem")
        );
        assert_eq!(
            party
                .name()
                .map(|name| name.value().into_owned())
                .as_deref(),
            Some("Democratic")
        );
        let abbreviation = party.abbreviation().and_then(|field| field.value());
        assert_eq!(
            abbreviation
                .map(|value| value.value().into_owned())
                .as_deref(),
            Some("D")
        );
        let website = party.website().and_then(|field| field.value());
        assert_eq!(
            website.map(|url| url.text().to_owned()).as_deref(),
            Some("https://dems.example")
        );
        let note = party.note().and_then(|field| field.body());
        assert_eq!(
            note.map(|body| body.value()).as_deref(),
            Some("Founded in 1828.")
        );
    }

    #[test]
    fn a_party_without_a_block_has_no_fields() {
        let party = party("party @dem \"Democratic\"\n");
        assert_eq!(party.abbreviation(), None);
        assert_eq!(party.website(), None);
        assert_eq!(party.note(), None);
    }

    #[test]
    fn a_party_missing_parts_of_its_line_has_none_of_them() {
        let party = party("party\n");
        assert_eq!(party.identifier(), None);
        assert_eq!(party.name(), None);
    }

    #[test]
    fn a_repeated_party_field_is_not_the_field() {
        let party = party("party @dem \"D\"\n  abbreviation \"A\"\n  abbreviation \"B\"\n");
        let abbreviation = party.abbreviation().and_then(|field| field.value());
        assert_eq!(
            abbreviation
                .map(|value| value.value().into_owned())
                .as_deref(),
            Some("A")
        );
    }

    #[test]
    fn only_a_party_node_is_a_party() {
        assert!(Party::can_cast(SyntaxKind::Party));
        assert!(!Party::can_cast(SyntaxKind::PartyRef));
    }
}
