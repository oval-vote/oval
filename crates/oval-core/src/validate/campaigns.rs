use std::collections::HashSet;

use oval_syntax::ast::{
    AstNode, Campaign, Candidate, Contest, Document, Identifier, Member, PartyRef,
};
use oval_syntax::{SyntaxKind, SyntaxNode, TextRange};

use super::index::{Entity, Index};
use super::{content_range, diagnostic, header_range, kept_nodes};
use crate::{Diagnostic, ValidationRule};

pub(super) fn misplaced_candidate_fields(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for campaign in kept_nodes(document.syntax()).filter_map(Campaign::cast) {
        let describes_a_candidate = !campaign.is_block_form()
            && campaign
                .members()
                .next()
                .is_some_and(|member| member.is_declaration());
        if describes_a_candidate {
            continue;
        }
        let designation = campaign.designation().map(|field| field.syntax().clone());
        let party = campaign.party().map(|field| field.syntax().clone());
        for field in designation.into_iter().chain(party) {
            if let Some(range) = content_range(&field) {
                diagnostics.push(diagnostic(ValidationRule::MisplacedCandidateField, range));
            }
        }
    }
}

pub(super) fn campaigns_without_members(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for campaign in kept_nodes(document.syntax()).filter_map(Campaign::cast) {
        let has_fields = campaign.website().is_some()
            || campaign.note().is_some()
            || campaign.designation().is_some()
            || campaign.party().is_some();
        if campaign.is_block_form() && has_fields && members(&campaign).is_empty() {
            push_line(
                ValidationRule::CampaignWithoutMembers,
                campaign.syntax(),
                diagnostics,
            );
        }
    }
}

pub(super) fn duplicate_members(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for campaign in kept_nodes(document.syntax()).filter_map(Campaign::cast) {
        let mut names = HashSet::new();
        for (name, member) in members(&campaign) {
            if !names.insert(name) {
                push_line(ValidationRule::DuplicateMember, &member, diagnostics);
            }
        }
    }
}

pub(super) fn members_in_two_campaigns(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for contest in kept_nodes(document.syntax()).filter_map(Contest::cast) {
        let mut earlier = HashSet::new();
        for campaign in contest.campaigns() {
            let members = members(&campaign);
            for (name, member) in &members {
                if earlier.contains(name) {
                    push_line(ValidationRule::MemberInTwoCampaigns, member, diagnostics);
                }
            }
            earlier.extend(members.into_iter().map(|(name, _)| name));
        }
    }
}

pub(super) fn parties_in_two_campaigns(
    document: &Document,
    index: &Index,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for contest in kept_nodes(document.syntax()).filter_map(Contest::cast) {
        let campaigns: Vec<Campaign> = contest.campaigns().collect();
        let mut later = Vec::new();
        for (position, campaign) in campaigns.iter().enumerate() {
            for candidate in campaign.members() {
                let Some(party) = candidate_party(&candidate, index) else {
                    continue;
                };
                let others = campaigns
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != position)
                    .flat_map(|(_, other)| members(other));
                for (name, member) in others {
                    if name == party {
                        let pair = [header_range(&member), header_range(candidate.syntax())];
                        later.extend(pair.into_iter().flatten().max_by_key(|range| range.start()));
                    }
                }
            }
        }
        later.sort_by_key(|range: &TextRange| range.start());
        later.dedup();
        for range in later {
            diagnostics.push(diagnostic(ValidationRule::PartyInTwoCampaigns, range));
        }
    }
}

pub(super) fn members(campaign: &Campaign) -> Vec<(String, SyntaxNode)> {
    let members = campaign.members().map(|member| member.syntax().clone());
    let party_members = campaign
        .party_members()
        .map(|member| member.syntax().clone());
    members
        .chain(party_members)
        .filter_map(|member| Some((name(&member)?, member)))
        .collect()
}

fn name(node: &SyntaxNode) -> Option<String> {
    let identifier = node.children().find_map(Identifier::cast)?;
    Some(identifier.name()?.text().to_owned())
}

fn candidate_party(member: &Member, index: &Index) -> Option<String> {
    let declaration = &index.declarations[index.resolve(&member.identifier()?)?];
    if declaration.entity != Entity::Candidate {
        return None;
    }
    let node = declaration.identifier.syntax().parent()?;
    let party: PartyRef = match node.kind() {
        SyntaxKind::Candidate => Candidate::cast(node)?.party()?,
        SyntaxKind::Member => {
            let member = Member::cast(node)?;
            match member.party() {
                Some(party) => party,
                None => Campaign::cast(member.syntax().parent()?)?.party()?,
            }
        }
        _ => return None,
    };
    name(party.syntax())
}

fn push_line(rule: ValidationRule, node: &SyntaxNode, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(range) = header_range(node) {
        diagnostics.push(diagnostic(rule, range));
    }
}

#[cfg(test)]
mod tests {
    use crate::validate::{reported, reported_as};

    #[test]
    fn a_block_form_campaign_with_fields_needs_a_member() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign incumbent  # c\n      website https://x.example\n    campaign party\n      note:\n        N.\n    campaign\n      bogus 1\n    campaign\n      @a \"A\"\n      website https://a.example\n    recommend no-position\n";
        assert_eq!(
            reported_as(text, "campaign-without-members"),
            [(88, 106), (148, 162)]
        );
    }

    #[test]
    fn each_member_repeating_an_earlier_one_of_its_campaign_is_a_duplicate() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign\n      @a \"A\" as \"P\"\n        designation \"D\"\n      @a as \"VP\"\n      @c \"C\"\n      @a\n    campaign party\n      @dem \"Dem\"\n      @dem\n    recommend no-position\n";
        assert_eq!(
            reported(text),
            [
                ("duplicate-member", 147, 157),
                ("duplicate-member", 177, 179),
                ("duplicate-member", 222, 226),
            ]
        );
    }

    #[test]
    fn each_member_repeating_one_of_an_earlier_campaign_is_in_two_campaigns() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @a \"A\"\n    campaign\n      @a\n      @a\n      @c \"C\"\n    campaign @c\n    recommend no-position\n";
        assert_eq!(
            reported(text),
            [
                ("member-in-two-campaigns", 123, 125),
                ("duplicate-member", 132, 134),
                ("member-in-two-campaigns", 132, 134),
                ("member-in-two-campaigns", 161, 163),
            ]
        );
    }

    #[test]
    fn the_later_of_a_candidate_and_a_member_naming_its_party_is_reported_once() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Dem\"\ncandidate @n \"N\"\n  party @dem\ncandidate @n2 \"N2\"\n  party @dem\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @n\n    campaign party @dem\n    campaign @m \"M\"\n      party @g \"G\"\n    campaign\n      @o \"O\"\n        party @dem\n    campaign party @g\n    recommend no-position\n  contest @k2 \"K2\" single\n    campaign @n\n    campaign @n2\n    campaign party @dem\n    campaign @dem\n    recommend no-position\n  contest @k3 \"K3\" single\n    campaign party @dem\n    campaign @o\n    recommend no-position\n";
        assert_eq!(
            reported_as(text, "party-in-two-campaigns"),
            [
                (198, 202),
                (261, 267),
                (306, 308),
                (413, 417),
                (431, 435),
                (525, 527)
            ]
        );
    }

    #[test]
    fn a_candidate_field_at_a_campaigns_level_that_describes_no_candidate_is_misplaced() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Dem\"\ncandidate @a \"A\"\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @a\n      designation \"S\"\n      party @dem\n    campaign\n      @c \"C\"\n      designation \"Q\"\n    campaign @d \"D\"\n      designation \"OK\"\n      party @dem\n    recommend no-position\n";
        assert_eq!(
            reported_as(text, "misplaced-candidate-field"),
            [(140, 155), (162, 172), (205, 220)]
        );
    }
}
