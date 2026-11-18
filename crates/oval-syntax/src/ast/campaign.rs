use crate::ast::{
    AstNode, Flag, MarkdownField, Member, PartyMember, PartyRef, StringField, UrlField, ast_node,
    support,
};
use crate::{Keyword, SyntaxKind};

ast_node! {
    /// A campaign in a contest, one or more candidates, or one or more parties, running together.
    ///
    /// A campaign is written in one of two forms. In the one-line form its only member is on the
    /// `campaign` line, and the block below holds the campaign's fields. In the block form the line
    /// has no member, and each member is a line in the block, with its own fields below it. A
    /// party campaign is written `campaign party`, and its members are parties.
    ///
    /// ```
    /// let text = r#"ballot @city "City"
    ///   contest @governor "Governor" single
    ///     campaign incumbent
    ///       @jlee "Jordan Lee" as "Governor"
    ///       @mpatel "Maya Patel" as "Lieutenant Governor"
    ///       website https://leepatel.example
    /// "#;
    /// let result = oval_syntax::parse(text)?;
    /// let ballot = result.document().ballots().next().expect("the document has a ballot");
    /// let Some(oval_syntax::ast::BallotItem::Contest(contest)) = ballot.items().next() else {
    ///     panic!("the ballot's first item is a contest");
    /// };
    /// let campaign = contest.campaigns().next().expect("the contest has a campaign");
    /// assert!(campaign.is_incumbent());
    /// assert!(campaign.is_block_form());
    /// assert_eq!(campaign.members().count(), 2);
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Campaign
}

impl Campaign {
    /// Returns the campaign's flags, in the order written.
    ///
    /// A flag written twice is set aside by the parser, so each flag appears at most once.
    pub fn flags(&self) -> impl Iterator<Item = Flag> + use<> {
        support::children(self.syntax())
    }

    /// Returns `true` if the campaign is flagged `incumbent`.
    pub fn is_incumbent(&self) -> bool {
        self.flags().any(|flag| flag.is_incumbent())
    }

    /// Returns `true` if the campaign is flagged `withdrawn`.
    pub fn is_withdrawn(&self) -> bool {
        self.flags().any(|flag| flag.is_withdrawn())
    }

    /// Returns `true` if the campaign is a party campaign, written `campaign party`.
    pub fn is_party(&self) -> bool {
        support::has_keyword(self.syntax(), Keyword::Party)
    }

    /// Returns `true` if the campaign is in block form, with no member on its `campaign` line.
    pub fn is_block_form(&self) -> bool {
        !self
            .syntax()
            .children()
            .any(|child| matches!(child.kind(), SyntaxKind::Member | SyntaxKind::PartyMember))
    }

    /// Returns the members of a candidate campaign, the one on its `campaign` line and then those
    /// in its block, in the order written. The first is the head of the ticket.
    ///
    /// A party campaign has no members of this kind. Its members are its
    /// [`party_members`](Self::party_members).
    pub fn members(&self) -> impl Iterator<Item = Member> + use<> {
        support::children(self.syntax()).chain(support::block_children(self.syntax()))
    }

    /// Returns the members of a party campaign, the one on its `campaign` line and then those in
    /// its block, in the order written. The first is the lead party.
    ///
    /// A candidate campaign has no members of this kind. Its members are its
    /// [`members`](Self::members).
    pub fn party_members(&self) -> impl Iterator<Item = PartyMember> + use<> {
        support::children(self.syntax()).chain(support::block_children(self.syntax()))
    }

    /// Returns the campaign's `website` field, or `None` if it has none.
    pub fn website(&self) -> Option<UrlField> {
        support::block_field(self.syntax(), Keyword::Website)
    }

    /// Returns the campaign's `note` field, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }

    /// Returns the `designation` field written at the campaign's level, or `None` if there is none.
    ///
    /// In the one-line form it describes the candidate declared on the `campaign` line. Anywhere
    /// else it is invalid, since it describes no candidate. A member's own designation is its
    /// [`Member::designation`].
    pub fn designation(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Designation)
    }

    /// Returns the `party` field written at the campaign's level, or `None` if there is none.
    ///
    /// Like [`designation`](Self::designation), it describes the candidate declared on the
    /// `campaign` line, and is invalid anywhere else.
    pub fn party(&self) -> Option<PartyRef> {
        support::block_children(self.syntax()).next()
    }
}

#[cfg(test)]
pub(super) fn first_campaign(lines: &str) -> Campaign {
    super::contest::first_contest(&format!("  contest @c \"C\" single\n{lines}"))
        .campaigns()
        .next()
        .expect("the contest has a campaign")
}

#[cfg(test)]
mod tests {
    use super::{Campaign, first_campaign};
    use crate::SyntaxKind;
    use crate::ast::AstNode;
    use crate::ast::contest::first_contest;

    fn member_ids(campaign: &Campaign) -> Vec<String> {
        campaign
            .members()
            .filter_map(|member| member.identifier())
            .filter_map(|id| id.name())
            .map(|name| name.text().to_owned())
            .collect()
    }

    fn party_member_ids(campaign: &Campaign) -> Vec<String> {
        campaign
            .party_members()
            .filter_map(|member| member.identifier())
            .filter_map(|id| id.name())
            .map(|name| name.text().to_owned())
            .collect()
    }

    #[test]
    fn a_one_line_campaign_has_its_member_and_fields() {
        let campaign = first_campaign(
            "    campaign incumbent @bsantos \"Beatriz Santos\"\n      designation \"Mayor\"\n      party @dem\n      website https://santos.example\n      note:\n        N.\n",
        );
        assert!(campaign.is_incumbent());
        assert!(!campaign.is_withdrawn());
        assert!(!campaign.is_party());
        assert!(!campaign.is_block_form());
        assert_eq!(member_ids(&campaign), ["bsantos"]);
        assert!(campaign.designation().is_some());
        assert!(campaign.party().is_some());
        assert!(campaign.website().is_some());
        assert!(campaign.note().is_some());
    }

    #[test]
    fn a_block_form_campaign_has_its_members_in_order() {
        let campaign = first_campaign(
            "    campaign withdrawn\n      @jlee \"Jordan Lee\" as \"Governor\"\n        designation \"Governor\"\n      @mpatel as \"Lieutenant Governor\"\n      website https://leepatel.example\n",
        );
        assert!(campaign.is_withdrawn());
        assert!(campaign.is_block_form());
        assert_eq!(member_ids(&campaign), ["jlee", "mpatel"]);
        assert_eq!(campaign.designation(), None);
        assert!(campaign.website().is_some());
    }

    #[test]
    fn a_one_line_campaign_can_have_members_in_its_block_too() {
        let campaign = first_campaign("    campaign @a \"A\"\n      @b \"B\"\n");
        assert!(!campaign.is_block_form());
        assert_eq!(member_ids(&campaign), ["a", "b"]);
    }

    #[test]
    fn a_party_campaign_has_party_members() {
        let one_line = first_campaign("    campaign incumbent party @dem\n");
        assert!(one_line.is_party());
        assert!(one_line.is_incumbent());
        assert_eq!(party_member_ids(&one_line), ["dem"]);
        assert_eq!(one_line.members().count(), 0);

        let block = first_campaign(
            "    campaign party\n      @green \"Green Party\"\n        abbreviation \"G\"\n      @labour\n      website https://redgreen.example\n",
        );
        assert!(block.is_party());
        assert!(block.is_block_form());
        assert_eq!(party_member_ids(&block), ["green", "labour"]);
        assert!(block.website().is_some());
    }

    #[test]
    fn a_candidate_campaign_has_no_party_members() {
        let campaign = first_campaign("    campaign @a \"A\"\n");
        assert_eq!(campaign.party_members().count(), 0);
    }

    #[test]
    fn a_repeated_flag_is_not_a_flag() {
        let campaign = first_campaign("    campaign incumbent incumbent @a\n");
        assert_eq!(campaign.flags().count(), 1);
    }

    #[test]
    fn a_contest_has_its_campaigns_in_order() {
        let contest = first_contest(
            "  contest @c \"C\" single\n    campaign @a\n    campaign party @dem\n    campaign\n      @b\n",
        );
        let kinds: Vec<bool> = contest
            .campaigns()
            .map(|campaign| campaign.is_party())
            .collect();
        assert_eq!(kinds, [false, true, false]);
    }

    #[test]
    fn only_a_campaign_node_is_a_campaign() {
        assert!(Campaign::can_cast(SyntaxKind::Campaign));
        assert!(!Campaign::can_cast(SyntaxKind::Member));
    }
}
