use std::collections::HashSet;

use oval_syntax::ast::{AstNode, Campaign, Member};
use oval_syntax::{SyntaxKind, TextRange};

use super::report;
use crate::index::Index;
use crate::{Diagnostic, ValidationRule};

/// Checks the campaign rules. A reference carries no block, a candidate is in one campaign per contest, a campaign lists each member once, and `designation` and `party` describe a candidate rather than a campaign.
pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        let mut in_contest: HashSet<String> = HashSet::new();
        for campaign in contest.campaigns() {
            let mut in_campaign: HashSet<String> = HashSet::new();
            for member in campaign.members() {
                member_block(&member, out);
                if let Some(id) = member.id() {
                    if !in_campaign.insert(id.name()) {
                        out.push(report(
                            ValidationRule::DuplicateMember,
                            member.range(),
                            format!("`@{}` is already a member of this campaign", id.name()),
                        ));
                    } else if !in_contest.insert(id.name()) {
                        out.push(report(
                            ValidationRule::CandidateInTwoCampaigns,
                            member.range(),
                            format!(
                                "`@{}` is already in another campaign of this contest",
                                id.name()
                            ),
                        ));
                    }
                }
            }
            candidate_fields(&campaign, out);
        }
    }
}

fn member_block(member: &Member, out: &mut Vec<Diagnostic>) {
    if member.is_declaration() {
        return;
    }
    let Some(block) = member
        .syntax()
        .children()
        .find(|n| n.kind() == SyntaxKind::Block)
    else {
        return;
    };
    let name = member.id().map(|id| id.name()).unwrap_or_default();
    let end = block
        .descendants_with_tokens()
        .filter_map(oval_syntax::SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia() && !token.text_range().is_empty())
        .last()
        .map_or_else(
            || block.text_range().end(),
            |token| token.text_range().end(),
        );
    out.push(report(
        ValidationRule::BlockOnReference,
        TextRange::new(block.text_range().start(), end),
        format!("`@{name}` refers to a candidate declared elsewhere, so it cannot have a block"),
    ));
}

fn candidate_fields(campaign: &Campaign, out: &mut Vec<Diagnostic>) {
    let header = campaign.syntax().children().find_map(Member::cast);
    let reason = match &header {
        None => "a campaign with its members in a block puts candidate fields under each member",
        Some(member) if !member.is_declaration() => {
            "the candidate is declared elsewhere, so its fields belong on that declaration"
        }
        Some(_) => return,
    };
    let fields = campaign
        .designation()
        .map(|f| ("designation", f.range()))
        .into_iter()
        .chain(campaign.party().map(|p| ("party", p.range())));
    for (name, range) in fields {
        out.push(report(
            ValidationRule::MisplacedCandidateField,
            range,
            format!("`{name}` is not allowed here: {reason}"),
        ));
    }
}
