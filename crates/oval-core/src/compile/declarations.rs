use oval_syntax::SyntaxNode;
use oval_syntax::ast::{self, AstNode, Campaign, PartyMember, PartyRef};

use super::{Declarations, id, markdown, string, url};
use crate::model::{Candidate, Contest, Member, Party};

pub(super) fn parties(root: &SyntaxNode, candidates: &[Candidate]) -> Option<Vec<Party>> {
    let mut parties = Vec::new();
    for node in root.descendants() {
        let fields = if let Some(party) = ast::Party::cast(node.clone()) {
            (
                party.identifier(),
                party.name(),
                party.abbreviation(),
                party.website(),
                party.note(),
            )
        } else if let Some(party) = PartyRef::cast(node.clone()).filter(PartyRef::is_declaration) {
            (
                party.identifier(),
                party.name(),
                party.abbreviation(),
                party.website(),
                party.note(),
            )
        } else if let Some(party) = PartyMember::cast(node).filter(PartyMember::is_declaration) {
            (
                party.identifier(),
                party.name(),
                party.abbreviation(),
                party.website(),
                party.note(),
            )
        } else {
            continue;
        };
        let (identifier, name, abbreviation, website, note) = fields;
        let id = id(identifier)?;
        parties.push(Party {
            candidates: candidates
                .iter()
                .filter(|candidate| candidate.party.as_ref() == Some(&id))
                .map(|candidate| candidate.id.clone())
                .collect(),
            id,
            name: name?.value().into_owned(),
            abbreviation: string(abbreviation),
            website: url(website),
            note: markdown(note),
            contests: Vec::new(),
        });
    }
    Some(parties)
}

pub(super) fn candidates(root: &SyntaxNode) -> Option<Vec<Candidate>> {
    let mut candidates = Vec::new();
    for node in root.descendants() {
        let fields = if let Some(candidate) = ast::Candidate::cast(node.clone()) {
            (
                candidate.identifier(),
                candidate.name(),
                candidate.designation(),
                candidate.party(),
                candidate.note(),
            )
        } else if let Some(member) = ast::Member::cast(node).filter(ast::Member::is_declaration) {
            let campaign = member.syntax().ancestors().find_map(Campaign::cast)?;
            if campaign.is_block_form() {
                (
                    member.identifier(),
                    member.name(),
                    member.designation(),
                    member.party(),
                    member.note(),
                )
            } else {
                (
                    member.identifier(),
                    member.name(),
                    campaign.designation(),
                    campaign.party(),
                    None,
                )
            }
        } else {
            continue;
        };
        let (identifier, name, designation, party, note) = fields;
        candidates.push(Candidate {
            id: id(identifier)?,
            name: name?.value().into_owned(),
            designation: string(designation),
            party: party.and_then(|party| id(party.identifier())),
            note: markdown(note),
            contests: Vec::new(),
        });
    }
    Some(candidates)
}

pub(super) fn contests(declarations: &mut Declarations, contests: &[&Contest]) {
    for contest in contests {
        for member in contest
            .campaigns
            .iter()
            .flat_map(|campaign| &campaign.members)
        {
            let entry = match member {
                Member::Candidate { candidate, .. } => declarations
                    .candidates
                    .iter_mut()
                    .find(|declared| declared.id == *candidate)
                    .map(|candidate| &mut candidate.contests),
                Member::Party { party } => declarations
                    .parties
                    .iter_mut()
                    .find(|declared| declared.id == *party)
                    .map(|party| &mut party.contests),
            };
            if let Some(contests) = entry {
                contests.push(contest.id.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::compile::compiled;

    #[test]
    fn parties_and_candidates_compile_in_document_order_wherever_they_are_declared() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Democratic\"\n  abbreviation \"D\"\n  website https://dem.example\n  note:\n    The party.\ncandidate @nreyes \"Nadia Reyes\"\n  designation \"Senator\"\n  party @dem\n  note:\n    Appointed.\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @bsantos \"Beatriz Santos\"\n      designation \"Mayor\"\n      party @green \"Green\"\n        abbreviation \"G\"\n      note:\n        The campaign's note.\n    campaign\n      @jlee \"Jordan Lee\" as \"Governor\"\n        party @dem\n        note:\n          First term.\n      @nreyes\n    recommend no-position\n  contest @a \"A\" single\n    campaign party @lab \"Labour\"\n      website https://lab.example\n    campaign party\n      @soc \"Social\"\n        abbreviation \"S\"\n      @dem\n    recommend no-position\n";
        let compiled = compiled(text);
        assert_eq!(
            compiled["parties"],
            json!([
                {
                    "id": "dem",
                    "name": "Democratic",
                    "abbreviation": "D",
                    "website": "https://dem.example",
                    "note": "The party.",
                    "candidates": ["nreyes", "jlee"],
                    "contests": ["a"],
                },
                {
                    "id": "green",
                    "name": "Green",
                    "abbreviation": "G",
                    "candidates": ["bsantos"],
                    "contests": [],
                },
                { "id": "lab", "name": "Labour", "candidates": [], "contests": ["a"] },
                {
                    "id": "soc",
                    "name": "Social",
                    "abbreviation": "S",
                    "candidates": [],
                    "contests": ["a"],
                },
            ])
        );
        assert_eq!(
            compiled["candidates"],
            json!([
                {
                    "id": "nreyes",
                    "name": "Nadia Reyes",
                    "designation": "Senator",
                    "party": "dem",
                    "note": "Appointed.",
                    "contests": ["k"],
                },
                {
                    "id": "bsantos",
                    "name": "Beatriz Santos",
                    "designation": "Mayor",
                    "party": "green",
                    "contests": ["k"],
                },
                {
                    "id": "jlee",
                    "name": "Jordan Lee",
                    "party": "dem",
                    "note": "First term.",
                    "contests": ["k"],
                },
            ])
        );
    }

    #[test]
    fn contests_list_where_each_candidate_and_party_is_a_member_in_ballot_order() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Democratic\"\nparty @lab \"Labour\"\ncandidate @nreyes \"Nadia Reyes\"\n  party @dem\ncandidate @cfox \"Carl Fox\"\nballot @s \"S\"\n  ballot @c \"C\"\n    contest @c1 \"C1\" single\n      campaign @nreyes\n      campaign party @lab\n      recommend no-position\n  section \"Offices\"\n    contest @s1 \"S1\" single\n      campaign\n        @cfox\n        @nreyes\n      recommend no-position\nballot @t \"T\"\n  contest @t1 \"T1\" single\n    campaign party\n      @lab\n      @green \"Green\"\n    campaign @nreyes\n    recommend no-position\n";
        let compiled = compiled(text);
        let contests = |list: &str, id: &str| {
            compiled[list]
                .as_array()
                .unwrap()
                .iter()
                .find(|declared| declared["id"] == id)
                .unwrap()["contests"]
                .clone()
        };
        assert_eq!(contests("candidates", "nreyes"), json!(["c1", "s1", "t1"]));
        assert_eq!(contests("candidates", "cfox"), json!(["s1"]));
        assert_eq!(contests("parties", "lab"), json!(["c1", "t1"]));
        assert_eq!(contests("parties", "green"), json!(["t1"]));
        assert_eq!(contests("parties", "dem"), json!([]));
    }
}
