use oval_syntax::ast;

use super::{Declarations, id, markdown, url};
use crate::model::{Campaign, CampaignKind, Member, Stance};

pub(super) fn campaigns(
    contest: &ast::Contest,
    declarations: &Declarations,
) -> Option<Vec<Campaign>> {
    contest
        .campaigns()
        .enumerate()
        .map(|(index, campaign)| self::campaign(index, &campaign, declarations))
        .collect()
}

fn campaign(
    index: usize,
    campaign: &ast::Campaign,
    declarations: &Declarations,
) -> Option<Campaign> {
    let mut members = Vec::new();
    let mut names = Vec::new();
    let mut ids = Vec::new();
    let kind = if campaign.is_party() {
        for member in campaign.party_members() {
            let party = declarations.party(&id(member.identifier())?)?;
            names.push(party.name.clone());
            ids.push(party.id.clone());
            members.push(Member::Party {
                party: party.id.clone(),
            });
        }
        CampaignKind::Parties
    } else {
        for member in campaign.members() {
            let candidate = declarations.candidate(&id(member.identifier())?)?;
            names.push(candidate.name.clone());
            ids.push(candidate.id.clone());
            members.push(Member::Candidate {
                candidate: candidate.id.clone(),
                role: member.role().map(|role| role.value().into_owned()),
                note: markdown(member.note()),
            });
        }
        CampaignKind::Candidates
    };
    let party = match kind {
        CampaignKind::Candidates => shared_party(&ids, declarations),
        CampaignKind::Parties => None,
    };
    Some(Campaign {
        index,
        kind,
        label: names.join(" / "),
        head: ids.first()?.clone(),
        party,
        members,
        incumbent: campaign.is_incumbent(),
        withdrawn: campaign.is_withdrawn(),
        website: url(campaign.website()),
        note: markdown(campaign.note()),
        stance: Stance::None,
    })
}

fn shared_party(candidates: &[String], declarations: &Declarations) -> Option<String> {
    let mut parties = candidates.iter().map(|id| {
        declarations
            .candidate(id)
            .and_then(|candidate| candidate.party.as_ref())
    });
    let first = parties.next()??;
    parties
        .all(|party| party == Some(first))
        .then(|| first.clone())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::compile::compiled;

    fn campaigns(contest: &str) -> Value {
        let text = format!(
            "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Democratic\"\nparty @lab \"Labour\"\ncandidate @nreyes \"Nadia Reyes\"\n  party @dem\ncandidate @cfox \"Carl Fox\"\nballot @b \"B\"\n  contest @k \"K\" single\n{contest}    recommend no-position\n"
        );
        compiled(&text)["ballots"][0]["items"][0]["campaigns"].clone()
    }

    #[test]
    fn a_one_line_campaign_has_its_member_and_the_campaign_fields() {
        assert_eq!(
            campaigns(
                "    campaign incumbent @bsantos \"Beatriz Santos\" as \"Mayor\"\n      designation \"Mayor of Ashford\"\n      party @dem\n      website https://santos.example\n      note:\n        Two terms.\n"
            ),
            json!([{
                "index": 0,
                "kind": "candidates",
                "label": "Beatriz Santos",
                "head": "bsantos",
                "party": "dem",
                "members": [{ "kind": "candidate", "candidate": "bsantos", "role": "Mayor" }],
                "incumbent": true,
                "withdrawn": false,
                "stance": { "position": "none" },
                "website": "https://santos.example",
                "note": "Two terms.",
            }])
        );
    }

    #[test]
    fn a_block_form_campaign_has_its_members_head_first() {
        assert_eq!(
            campaigns(
                "    campaign withdrawn\n      @jlee \"Jordan Lee\" as \"Governor\"\n        party @dem\n        note:\n          First term.\n      @nreyes as \"Lieutenant Governor\"\n      note:\n        The ticket.\n"
            ),
            json!([{
                "index": 0,
                "kind": "candidates",
                "label": "Jordan Lee / Nadia Reyes",
                "head": "jlee",
                "party": "dem",
                "members": [
                    {
                        "kind": "candidate",
                        "candidate": "jlee",
                        "role": "Governor",
                        "note": "First term.",
                    },
                    { "kind": "candidate", "candidate": "nreyes", "role": "Lieutenant Governor" },
                ],
                "incumbent": false,
                "withdrawn": true,
                "stance": { "position": "none" },
                "note": "The ticket.",
            }])
        );
    }

    #[test]
    fn a_campaign_has_a_party_only_when_every_member_shares_it() {
        let party = |campaign: &str| {
            campaigns(campaign)[0]
                .get("party")
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        assert_eq!(party("    campaign @nreyes\n").as_deref(), Some("dem"));
        assert_eq!(
            party("    campaign\n      @nreyes\n      @jlee \"Jordan Lee\"\n        party @dem\n")
                .as_deref(),
            Some("dem")
        );
        assert_eq!(party("    campaign\n      @nreyes\n      @cfox\n"), None);
        assert_eq!(party("    campaign\n      @cfox\n      @nreyes\n"), None);
        assert_eq!(
            party("    campaign\n      @nreyes\n      @ofox \"Omar Fox\"\n        party @lab\n"),
            None
        );
        assert_eq!(party("    campaign @cfox\n"), None);
    }

    #[test]
    fn a_party_campaign_has_parties_as_members_and_no_party() {
        assert_eq!(
            campaigns(
                "    campaign party @dem\n    campaign party\n      @green \"Green\"\n        abbreviation \"G\"\n      @lab\n      website https://redgreen.example\n    campaign @cfox\n"
            ),
            json!([
                {
                    "index": 0,
                    "kind": "parties",
                    "label": "Democratic",
                    "head": "dem",
                    "members": [{ "kind": "party", "party": "dem" }],
                    "incumbent": false,
                    "withdrawn": false,
                    "stance": { "position": "none" },
                },
                {
                    "index": 1,
                    "kind": "parties",
                    "label": "Green / Labour",
                    "head": "green",
                    "members": [
                        { "kind": "party", "party": "green" },
                        { "kind": "party", "party": "lab" },
                    ],
                    "incumbent": false,
                    "withdrawn": false,
                    "stance": { "position": "none" },
                    "website": "https://redgreen.example",
                },
                {
                    "index": 2,
                    "kind": "candidates",
                    "label": "Carl Fox",
                    "head": "cfox",
                    "members": [{ "kind": "candidate", "candidate": "cfox" }],
                    "incumbent": false,
                    "withdrawn": false,
                    "stance": { "position": "none" },
                },
            ])
        );
    }
}
