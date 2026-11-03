use std::collections::HashMap;

use oval_syntax::ast::{AstNode, Contest, ContestKind, ContestRecommend};

use super::{error, warning};
use crate::index::{EntityKind, Index};
use crate::{Diagnostic, Rule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        let campaign_of = campaign_memberships(contest);
        for recommendation in contest.recommendations() {
            in_contest(index, &recommendation, &campaign_of, out);
            shape(&recommendation, &campaign_of, out);
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
    let opposed: usize = recommendation
        .oppose()
        .map(|o| o.candidates().count())
        .sum();
    match kind {
        ContestKind::Single => {
            for tier in tiers.iter().skip(1) {
                out.push(error(
                    Rule::KindSpecificRecommendation,
                    tier.range(),
                    "a vote-for-one contest takes one `support` line; put interchangeable choices on it together".to_owned(),
                ));
            }
        }
        ContestKind::Multiple { seats: Some(seats) } if named > seats as usize => {
            out.push(warning(
                Rule::KindSpecificRecommendation,
                recommendation.range(),
                format!("this recommendation supports {named} candidates for {seats} seats; later tiers are fallbacks"),
            ));
        }
        ContestKind::Ranked { max: Some(max) } if named > max as usize => {
            let mut running = 0;
            for tier in &tiers {
                running += tier.candidates().count();
                if running > max as usize {
                    out.push(error(
                        Rule::KindSpecificRecommendation,
                        tier.range(),
                        format!("this recommendation ranks {named} candidates but voters may rank only {max}"),
                    ));
                    break;
                }
            }
        }
        ContestKind::Retention if named + opposed > 1 || (named == 1 && opposed == 1) => {
            out.push(error(
                Rule::KindSpecificRecommendation,
                recommendation.range(),
                "a retention recommendation is one `support` or one `oppose` of the single campaign".to_owned(),
            ));
        }
        _ => {}
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
            .resolve(id.name())
            .is_some_and(|d| d.kind == EntityKind::Candidate);
        if declared_candidate && !campaign_of.contains_key(id.name()) {
            out.push(error(
                Rule::RecommendationReferencesInContest,
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
            .map(|id| id.name().to_owned())
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
        if let Some(head) = head_of.get(id.name()) {
            out.push(warning(
                Rule::HeadOfTicketReferences,
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
                map.entry(id.name().to_owned()).or_insert(i);
            }
        }
    }
    map
}

fn shape(
    recommendation: &ContestRecommend,
    campaign_of: &HashMap<String, usize>,
    out: &mut Vec<Diagnostic>,
) {
    for extra in recommendation.oppose().skip(1) {
        out.push(error(
            Rule::RecommendationShape,
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
    let mut seen: HashMap<usize, String> = HashMap::new();
    for id in named {
        let Some(campaign) = campaign_of.get(id.name()) else {
            continue;
        };
        match seen.get(campaign) {
            None => {
                seen.insert(*campaign, id.name().to_owned());
            }
            Some(first) if first == id.name() => out.push(error(
                Rule::RecommendationShape,
                id.range(),
                format!("`@{}` is named more than once", id.name()),
            )),
            Some(first) => out.push(error(
                Rule::RecommendationShape,
                id.range(),
                format!(
                    "`@{}` is on the same ticket as `@{first}`, which is already named",
                    id.name()
                ),
            )),
        }
    }
}
