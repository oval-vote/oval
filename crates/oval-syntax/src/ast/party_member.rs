use crate::Keyword;
use crate::ast::{
    AstNode, Identifier, MarkdownField, StringField, StringLiteral, UrlField, ast_node, support,
};

ast_node! {
    /// A member of a party campaign, which refers to a party, such as `@dem`, or declares one in
    /// place, such as `@green "Green Party"`.
    ///
    /// A member on its own line in a campaign's block may have party fields in the block below it.
    /// A block below a reference is still read, but it is invalid, since a reference has nothing
    /// to declare.
    PartyMember
}

impl PartyMember {
    /// Returns the identifier of the party referred to or declared, or `None` if there is none.
    pub fn identifier(&self) -> Option<Identifier> {
        support::child(self.syntax())
    }

    /// Returns the name of the party declared in place, or `None` if the member is a reference.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns `true` if the member declares a party in place, which it does when it has a name.
    pub fn is_declaration(&self) -> bool {
        self.name().is_some()
    }

    /// Returns the `abbreviation` field in the member's own block, or `None` if it has none.
    pub fn abbreviation(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Abbreviation)
    }

    /// Returns the `website` field in the member's own block, or `None` if it has none.
    pub fn website(&self) -> Option<UrlField> {
        support::block_field(self.syntax(), Keyword::Website)
    }

    /// Returns the `note` field in the member's own block, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }
}

#[cfg(test)]
mod tests {
    use super::PartyMember;
    use crate::SyntaxKind;
    use crate::ast::AstNode;
    use crate::ast::campaign::first_campaign;

    fn members(lines: &str) -> Vec<PartyMember> {
        first_campaign(lines).party_members().collect()
    }

    #[test]
    fn a_declared_party_member_has_its_name_and_fields() {
        let members = members(
            "    campaign party\n      @green \"Green Party\"\n        abbreviation \"G\"\n        website https://greens.example\n        note:\n          N.\n",
        );
        let member = &members[0];
        assert!(member.is_declaration());
        let name = member.name().map(|name| name.value().into_owned());
        assert_eq!(name.as_deref(), Some("Green Party"));
        assert!(member.abbreviation().is_some());
        assert!(member.website().is_some());
        assert!(member.note().is_some());
    }

    #[test]
    fn a_party_reference_has_no_name() {
        let members = members("    campaign party @dem\n");
        assert!(!members[0].is_declaration());
        assert_eq!(members[0].abbreviation(), None);
    }

    #[test]
    fn only_a_party_member_node_is_a_party_member() {
        assert!(PartyMember::can_cast(SyntaxKind::PartyMember));
        assert!(!PartyMember::can_cast(SyntaxKind::Member));
    }
}
