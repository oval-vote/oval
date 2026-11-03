use oval_syntax::SyntaxKind;
use oval_syntax::ast::{AstNode, Campaign, Member};

use super::error;
use crate::index::Index;
use crate::{Diagnostic, Rule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        for campaign in contest.campaigns() {
            for member in campaign.members() {
                member_block(&member, out);
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
    let name = member
        .id()
        .map(|id| id.name().to_owned())
        .unwrap_or_default();
    out.push(error(
        Rule::MemberBlocksOnlyOnDeclarations,
        block.text_range(),
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
        out.push(error(
            Rule::CandidateFieldsInPlace,
            range,
            format!("`{name}` is not allowed here: {reason}"),
        ));
    }
}
