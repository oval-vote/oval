use oval_syntax::ast;

use super::{id, markdown};
use crate::model::{
    Campaign, CampaignKind, Choice, ContestPosition, ContestRecommendation, Member, Stance,
};

pub(super) fn recommendation(
    contest: &ast::Contest,
    campaigns: &mut [Campaign],
) -> Option<ContestRecommendation> {
    let recommendation = contest.recommendations().next()?;
    let position = match recommendation.no_stance() {
        Some(no_stance) if no_stance.is_abstain() => ContestPosition::Abstain,
        Some(_) => ContestPosition::NoPosition,
        None => ContestPosition::Stance,
    };
    let mut support = Vec::new();
    for (tier, line) in recommendation.supports().enumerate() {
        let choices = choices(line.choices(), campaigns)?;
        mark(&choices, campaigns, Stance::Support { tier });
        support.push(choices);
    }
    let mut oppose = Vec::new();
    for line in recommendation.opposes() {
        let choices = choices(line.choices(), campaigns)?;
        mark(&choices, campaigns, Stance::Oppose);
        oppose.extend(choices);
    }
    Some(ContestRecommendation {
        position,
        support,
        oppose,
        justification: markdown(recommendation.justification()),
        note: markdown(recommendation.note()),
    })
}

fn choices(
    choices: impl Iterator<Item = ast::Choice>,
    campaigns: &[Campaign],
) -> Option<Vec<Choice>> {
    choices
        .map(|choice| self::choice(choice, campaigns))
        .collect()
}

fn choice(choice: ast::Choice, campaigns: &[Campaign]) -> Option<Choice> {
    match choice {
        ast::Choice::Reference(identifier) => {
            let id = id(Some(identifier))?;
            let campaign = campaigns
                .iter()
                .find(|campaign| campaign.members.iter().any(|member| named(member, &id)))?;
            Some(match campaign.kind {
                CampaignKind::Candidates => Choice::Candidate {
                    candidate: campaign.head.clone(),
                    campaign: campaign.index,
                },
                CampaignKind::Parties => Choice::Party {
                    party: campaign.head.clone(),
                    campaign: campaign.index,
                },
            })
        }
        ast::Choice::WriteIn(write_in) => Some(Choice::WriteIn {
            name: write_in.name().map(|name| name.value().into_owned()),
        }),
    }
}

fn named(member: &Member, id: &str) -> bool {
    match member {
        Member::Candidate { candidate, .. } => candidate == id,
        Member::Party { party } => party == id,
    }
}

fn mark(choices: &[Choice], campaigns: &mut [Campaign], stance: Stance) {
    for choice in choices {
        if let Choice::Candidate { campaign, .. } | Choice::Party { campaign, .. } = choice {
            campaigns[*campaign].stance = stance;
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::compile::compiled;

    fn contest(text: &str) -> Value {
        let text = format!(
            "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n{text}"
        );
        compiled(&text)["ballots"][0]["items"][0].clone()
    }

    fn stances(contest: &Value) -> Vec<Value> {
        contest["campaigns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|campaign| campaign["stance"].clone())
            .collect()
    }

    #[test]
    fn a_stance_has_its_tiers_and_oppose_with_each_reference_resolved_to_its_campaign() {
        let contest = contest(
            "  extensible contest @k \"K\" ranked\n    campaign\n      @jlee \"Jordan Lee\"\n      @mpatel \"Maya Patel\"\n    campaign @cfox \"Carl Fox\"\n    campaign party\n      @dem \"Democratic\"\n      @lab \"Labour\"\n    campaign @dwright \"Dana Wright\"\n    recommend\n      justification:\n        Why.\n      support @mpatel\n      support @lab write-in \"Jane Doe\"\n      oppose @cfox write-in\n      note:\n        Close.\n",
        );
        assert_eq!(
            contest["recommendation"],
            json!({
                "position": "stance",
                "support": [
                    [{ "kind": "candidate", "candidate": "jlee", "campaign": 0 }],
                    [
                        { "kind": "party", "party": "dem", "campaign": 2 },
                        { "kind": "write_in", "name": "Jane Doe" },
                    ],
                ],
                "oppose": [
                    { "kind": "candidate", "candidate": "cfox", "campaign": 1 },
                    { "kind": "write_in" },
                ],
                "justification": "Why.",
                "note": "Close.",
            })
        );
        assert_eq!(
            stances(&contest),
            [
                json!({ "position": "support", "tier": 0 }),
                json!({ "position": "oppose" }),
                json!({ "position": "support", "tier": 1 }),
                json!({ "position": "none" }),
            ]
        );
    }

    #[test]
    fn without_a_stance_there_are_no_choices_and_every_campaign_is_unnamed() {
        for (position, justification) in [("no-position", "Why."), ("abstain", "Skip it.")] {
            let contest = contest(&format!(
                "  contest @k \"K\" single\n    campaign @cfox \"Carl Fox\"\n    campaign @dwright \"Dana Wright\"\n    recommend {position}\n      justification:\n        {justification}\n"
            ));
            assert_eq!(
                contest["recommendation"],
                json!({
                    "position": position,
                    "support": [],
                    "oppose": [],
                    "justification": justification,
                }),
                "{position}"
            );
            assert_eq!(
                stances(&contest),
                [json!({ "position": "none" }), json!({ "position": "none" })]
            );
        }
    }
}
