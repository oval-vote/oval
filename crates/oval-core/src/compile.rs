use oval_syntax::SyntaxNode;
use oval_syntax::ast::{
    self, AstNode, AstToken, Campaign, Document, Identifier, MarkdownField, Member, PartyMember,
    PartyRef, StringField, UrlField,
};

use crate::model::{Candidate, Compiled, Guide, Party};

pub(crate) fn compile(document: &Document) -> Option<Compiled> {
    let candidates = candidates(document.syntax())?;
    let parties = parties(document.syntax(), &candidates)?;
    Some(Compiled {
        oval: document.version_marker()?.version()?.value()?,
        guide: Guide {
            name: string(document.guide())?,
            election: string(document.election())?,
            date: document.date()?.value()?.text().to_owned(),
            note: markdown(document.note()),
        },
        parties,
        candidates,
    })
}

fn parties(root: &SyntaxNode, candidates: &[Candidate]) -> Option<Vec<Party>> {
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
        });
    }
    Some(parties)
}

fn candidates(root: &SyntaxNode) -> Option<Vec<Candidate>> {
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
        } else if let Some(member) = Member::cast(node).filter(Member::is_declaration) {
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
        });
    }
    Some(candidates)
}

fn id(identifier: Option<Identifier>) -> Option<String> {
    Some(identifier?.name()?.text().to_owned())
}

fn string(field: Option<StringField>) -> Option<String> {
    Some(field?.value()?.value().into_owned())
}

fn url(field: Option<UrlField>) -> Option<String> {
    Some(field?.value()?.text().to_owned())
}

fn markdown(field: Option<MarkdownField>) -> Option<String> {
    Some(field?.body()?.value())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::check;
    use crate::validate::VALID;

    fn compiled(text: &str) -> serde_json::Value {
        let compiled = check(text)
            .unwrap()
            .compile()
            .expect("the document compiles");
        serde_json::from_str(&compiled.to_json()).unwrap()
    }

    #[test]
    fn the_guide_fields_compile_with_their_values() {
        let text = VALID
            .replacen("\"G\"", "\"Franklin \\\"Votes\\\"\"", 1)
            .replacen(
                "date 2026-11-03\n",
                "date 2026-11-03\nnote:\n    Vote *early*.\n\n      Really.\n",
                1,
            );
        let compiled = compiled(&text);
        assert_eq!(compiled["oval"], 1);
        assert_eq!(
            compiled["guide"],
            json!({
                "name": "Franklin \"Votes\"",
                "election": "E",
                "date": "2026-11-03",
                "note": "Vote *early*.\n\n  Really.",
            })
        );
    }

    #[test]
    fn a_note_the_author_did_not_write_is_left_out() {
        assert_eq!(
            compiled(VALID)["guide"],
            json!({ "name": "G", "election": "E", "date": "2026-11-03" })
        );
    }

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
                },
                { "id": "green", "name": "Green", "abbreviation": "G", "candidates": ["bsantos"] },
                { "id": "lab", "name": "Labour", "candidates": [] },
                { "id": "soc", "name": "Social", "abbreviation": "S", "candidates": [] },
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
                },
                { "id": "bsantos", "name": "Beatriz Santos", "designation": "Mayor", "party": "green" },
                { "id": "jlee", "name": "Jordan Lee", "party": "dem", "note": "First term." },
            ])
        );
    }
}
