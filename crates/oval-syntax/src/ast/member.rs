use crate::Keyword;
use crate::SyntaxKind;
use crate::ast::{
    AstNode, Identifier, MarkdownField, PartyRef, StringField, StringLiteral, ast_node, support,
};

ast_node! {
    /// A member of a candidate campaign, which refers to a candidate, such as `@jlee`, or declares
    /// one in place, such as `@jlee "Jordan Lee"`, and may give the member's role, such as
    /// `as "Governor"`.
    ///
    /// A member on its own line in a campaign's block may have candidate fields in the block below
    /// it. A member on the `campaign` line has none of its own, since the block below that line is
    /// the campaign's. A block below a reference is still read, but it is invalid, since a
    /// reference has nothing to declare.
    Member
}

impl Member {
    /// Returns the identifier of the candidate referred to or declared, or `None` if there is none.
    pub fn identifier(&self) -> Option<Identifier> {
        support::child(self.syntax())
    }

    /// Returns the name of the candidate declared in place, or `None` if the member is a
    /// reference.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns `true` if the member declares a candidate in place, which it does when it has a
    /// name.
    pub fn is_declaration(&self) -> bool {
        self.name().is_some()
    }

    /// Returns the member's role on the ticket, the string after `as`, or `None` if it has none.
    pub fn role(&self) -> Option<StringLiteral> {
        self.syntax()
            .children()
            .find(|child| child.kind() == SyntaxKind::MemberRole)
            .and_then(|role| support::token(&role))
    }

    /// Returns the `designation` field in the member's own block, or `None` if it has none.
    pub fn designation(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Designation)
    }

    /// Returns the `party` field in the member's own block, or `None` if it has none.
    pub fn party(&self) -> Option<PartyRef> {
        support::block_children(self.syntax()).next()
    }

    /// Returns the `note` field in the member's own block, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }
}

#[cfg(test)]
mod tests {
    use super::Member;
    use crate::SyntaxKind;
    use crate::ast::AstNode;
    use crate::ast::campaign::first_campaign;

    fn members(lines: &str) -> Vec<Member> {
        first_campaign(lines).members().collect()
    }

    fn role(member: &Member) -> Option<String> {
        member.role().map(|role| role.value().into_owned())
    }

    #[test]
    fn a_member_line_has_its_name_role_and_fields() {
        let members = members(
            "    campaign\n      @jlee \"Jordan Lee\" as \"Governor\"\n        designation \"Governor of Franklin\"\n        party @dem\n        note:\n          N.\n",
        );
        let member = &members[0];
        let name = member.name().map(|name| name.value().into_owned());
        assert_eq!(name.as_deref(), Some("Jordan Lee"));
        assert!(member.is_declaration());
        assert_eq!(role(member).as_deref(), Some("Governor"));
        assert!(member.designation().is_some());
        assert!(member.party().is_some());
        assert!(member.note().is_some());
    }

    #[test]
    fn a_reference_has_no_name_and_may_have_a_role() {
        let members = members("    campaign\n      @mpatel as \"Lieutenant Governor\"\n");
        assert!(!members[0].is_declaration());
        assert_eq!(role(&members[0]).as_deref(), Some("Lieutenant Governor"));
    }

    #[test]
    fn a_member_on_the_campaign_line_has_no_fields_of_its_own() {
        let members = members("    campaign @a \"A\"\n      designation \"D\"\n");
        assert_eq!(members[0].designation(), None);
    }

    #[test]
    fn a_role_without_a_string_is_absent() {
        let members = members("    campaign @a as\n");
        assert_eq!(members[0].role(), None);
    }

    #[test]
    fn only_a_member_node_is_a_member() {
        assert!(Member::can_cast(SyntaxKind::Member));
        assert!(!Member::can_cast(SyntaxKind::PartyMember));
    }
}
