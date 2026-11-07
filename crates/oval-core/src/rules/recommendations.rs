use std::collections::{HashMap, HashSet};

use oval_syntax::ast::{AstNode, Contest, ContestKind, ContestRecommend, Oppose, Support};
use oval_syntax::{SyntaxKind, TextRange};

use super::report;
use crate::index::{EntityKind, Index};
use crate::{Diagnostic, ValidationRule};

/// Checks the recommendations of contests. References name campaigns in that contest by their head, each campaign is named once, and the tiers and counts fit the contest's kind.
pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        let campaign_of = campaign_memberships(contest);
        for recommendation in contest.recommendations() {
            in_contest(index, &recommendation, &campaign_of, out);
            shape(&recommendation, out);
            head_of_ticket(contest, &recommendation, out);
            if let Some(kind) = contest.kind() {
                kind_specific(kind, &recommendation, out);
            }
        }
    }
}

fn kind_specific(kind: ContestKind, recommendation: &ContestRecommend, out: &mut Vec<Diagnostic>) {
    let tiers: Vec<_> = recommendation.support().collect();
    let named: usize = tiers.iter().map(|t| t.candidates().count()).sum();
    match kind {
        ContestKind::Single => {
            for tier in tiers.iter().skip(1) {
                out.push(report(
                    ValidationRule::TiersInSingle,
                    tier.range(),
                    "a vote-for-one contest takes one `support` line; put interchangeable choices on it together".to_owned(),
                ));
            }
        }
        ContestKind::Multiple { seats: Some(seats) } if named > seats as usize => {
            if let Some(beyond) = reference_beyond(&tiers, seats as usize) {
                out.push(report(
                    ValidationRule::SupportOverCount,
                    beyond,
                    format!("this recommendation supports {named} candidates for {seats} seats; later tiers are fallbacks"),
                ));
            }
        }
        ContestKind::Ranked { max: Some(max) } if named > max as usize => {
            if let Some(beyond) = reference_beyond(&tiers, max as usize) {
                out.push(report(
                    ValidationRule::SupportOverMax,
                    beyond,
                    format!("this recommendation ranks {named} candidates but voters may rank only {max}"),
                ));
            }
        }
        ContestKind::Retention => retention_stance(recommendation, out),
        _ => {}
    }
}

fn reference_beyond(tiers: &[Support], limit: usize) -> Option<TextRange> {
    tiers
        .iter()
        .flat_map(|tier| tier.candidates().collect::<Vec<_>>())
        .nth(limit)
        .map(|candidate| candidate.range())
}

fn retention_stance(recommendation: &ContestRecommend, out: &mut Vec<Diagnostic>) {
    let lines = recommendation
        .syntax()
        .children()
        .filter(|node| node.kind() == SyntaxKind::Block)
        .flat_map(|block| block.children())
        .filter_map(|node| {
            Support::cast(node.clone())
                .map(|support| (support.range(), support.candidates().count()))
                .or_else(|| {
                    Oppose::cast(node).map(|oppose| (oppose.range(), oppose.candidates().count()))
                })
        });
    let mut references = 0;
    for (range, count) in lines {
        references += count;
        if references > 1 {
            out.push(report(
                ValidationRule::RetentionStance,
                range,
                "a retention recommendation is one `support` or one `oppose` of the single campaign".to_owned(),
            ));
            return;
        }
    }
}

fn in_contest(
    index: &Index,
    recommendation: &ContestRecommend,
    campaign_of: &HashMap<String, usize>,
    out: &mut Vec<Diagnostic>,
) {
    let named = recommendation
        .support()
        .flat_map(|tier| tier.candidates().collect::<Vec<_>>())
        .chain(
            recommendation
                .oppose()
                .flat_map(|o| o.candidates().collect::<Vec<_>>()),
        );
    for id in named {
        let declared_candidate = index
            .resolve(&id.name())
            .is_some_and(|d| d.kind == EntityKind::Candidate);
        if declared_candidate && !campaign_of.contains_key(&id.name()) {
            out.push(report(
                ValidationRule::CampaignNotInContest,
                id.range(),
                format!("`@{}` is not in any campaign of this contest", id.name()),
            ));
        }
    }
}

fn head_of_ticket(contest: &Contest, recommendation: &ContestRecommend, out: &mut Vec<Diagnostic>) {
    let mut head_of: HashMap<String, String> = HashMap::new();
    for campaign in contest.campaigns() {
        let names: Vec<String> = campaign
            .members()
            .filter_map(|m| m.id())
            .map(|id| id.name())
            .collect();
        if let [head, rest @ ..] = names.as_slice() {
            for other in rest {
                head_of.entry(other.clone()).or_insert_with(|| head.clone());
            }
        }
    }
    let named = recommendation
        .support()
        .flat_map(|tier| tier.candidates().collect::<Vec<_>>())
        .chain(
            recommendation
                .oppose()
                .flat_map(|o| o.candidates().collect::<Vec<_>>()),
        );
    for id in named {
        if let Some(head) = head_of.get(&id.name()) {
            out.push(report(
                ValidationRule::NonHeadReference,
                id.range(),
                format!(
                    "`@{}` is not the head of its ticket; name `@{head}` instead",
                    id.name()
                ),
            ));
        }
    }
}

fn campaign_memberships(contest: &Contest) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (i, campaign) in contest.campaigns().enumerate() {
        for member in campaign.members() {
            if let Some(id) = member.id() {
                map.entry(id.name()).or_insert(i);
            }
        }
    }
    map
}

fn shape(recommendation: &ContestRecommend, out: &mut Vec<Diagnostic>) {
    for extra in recommendation.oppose().skip(1) {
        out.push(report(
            ValidationRule::RepeatedOppose,
            extra.range(),
            "a recommendation may have only one `oppose` line".to_owned(),
        ));
    }
    let named = recommendation
        .support()
        .flat_map(|tier| tier.candidates().collect::<Vec<_>>())
        .chain(
            recommendation
                .oppose()
                .flat_map(|o| o.candidates().collect::<Vec<_>>()),
        );
    let mut seen: HashSet<String> = HashSet::new();
    for id in named {
        if !seen.insert(id.name()) {
            out.push(report(
                ValidationRule::RepeatedStance,
                id.range(),
                format!("`@{}` is named more than once", id.name()),
            ));
        }
    }
}
