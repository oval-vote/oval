use std::collections::HashMap;

use oval_syntax::ast::{AstNode, Contest, ContestRecommend};

use super::error;
use crate::index::{EntityKind, Index};
use crate::{Diagnostic, Rule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        let campaign_of = campaign_memberships(contest);
        for recommendation in contest.recommendations() {
            in_contest(index, &recommendation, &campaign_of, out);
            shape(&recommendation, &campaign_of, out);
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
