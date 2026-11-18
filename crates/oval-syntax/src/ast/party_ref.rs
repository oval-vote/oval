use crate::Keyword;
use crate::ast::{
    AstNode, Identifier, MarkdownField, StringField, StringLiteral, UrlField, ast_node, support,
};

ast_node! {
    /// A candidate's `party` field, which refers to a party, such as `party @dem`, or declares one
    /// in place, such as `party @green "Green"`.
    ///
    /// A party declared in place has a name, and may have party fields in the block below it. A
    /// reference has no name. A block below a reference is still read, since the parser keeps it,
    /// but it is invalid, because a reference has nothing to declare.
    ///
    /// ```
    /// let text = "candidate @c \"C\"\n  party @green \"Green\"\n    abbreviation \"G\"\n";
    /// let result = oval_syntax::parse(text)?;
    /// let candidate = result.document().candidates().next().expect("a candidate");
    /// let party = candidate.party().expect("the candidate has a party");
    /// assert!(party.is_declaration());
    /// assert!(party.abbreviation().is_some());
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    PartyRef
}

impl PartyRef {
    /// Returns the identifier of the party referred to or declared, or `None` if the line has none.
    pub fn identifier(&self) -> Option<Identifier> {
        support::child(self.syntax())
    }

    /// Returns the name of the party declared in place, or `None` if the field is a reference.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns `true` if the field declares a party in place, which it does when it has a name.
    pub fn is_declaration(&self) -> bool {
        self.name().is_some()
    }

    /// Returns the declared party's `abbreviation` field, or `None` if it has none.
    pub fn abbreviation(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Abbreviation)
    }

    /// Returns the declared party's `website` field, or `None` if it has none.
    pub fn website(&self) -> Option<UrlField> {
        support::block_field(self.syntax(), Keyword::Website)
    }

    /// Returns the declared party's `note` field, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }
}

#[cfg(test)]
mod tests {
    use super::PartyRef;
    use crate::ast::AstNode;
    use crate::{SyntaxKind, parse};

    fn party_ref(lines: &str) -> PartyRef {
        parse(&format!("candidate @c \"C\"\n{lines}"))
            .unwrap()
            .document()
            .candidates()
            .next()
            .and_then(|candidate| candidate.party())
            .expect("the candidate has a party field")
    }

    fn identifier(party: &PartyRef) -> Option<String> {
        party
            .identifier()
            .and_then(|id| id.name())
            .map(|name| name.text().to_owned())
    }

    #[test]
    fn a_reference_has_an_identifier_and_no_name() {
        let party = party_ref("  party @dem\n");
        assert_eq!(identifier(&party).as_deref(), Some("dem"));
        assert_eq!(party.name(), None);
        assert!(!party.is_declaration());
    }

    #[test]
    fn a_declaration_has_a_name_and_fields() {
        let party = party_ref(
            "  party @green \"Green\"\n    abbreviation \"G\"\n    website https://greens.example\n    note:\n      N.\n",
        );
        assert_eq!(identifier(&party).as_deref(), Some("green"));
        assert_eq!(
            party
                .name()
                .map(|name| name.value().into_owned())
                .as_deref(),
            Some("Green")
        );
        assert!(party.is_declaration());
        assert!(party.abbreviation().is_some());
        assert!(party.website().is_some());
        assert!(party.note().is_some());
    }

    #[test]
    fn a_reference_keeps_the_fields_of_a_block_below_it() {
        let party = party_ref("  party @dem\n    abbreviation \"D\"\n");
        assert!(!party.is_declaration());
        assert!(party.abbreviation().is_some());
    }

    #[test]
    fn only_a_party_ref_node_is_a_party_ref() {
        assert!(PartyRef::can_cast(SyntaxKind::PartyRef));
        assert!(!PartyRef::can_cast(SyntaxKind::Party));
    }
}
