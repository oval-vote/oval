use oval_syntax::ast::{
    self, AstNode, AstToken, Campaign, Document, Identifier, MarkdownField, Member, PartyMember,
    PartyRef, StringField, UrlField,
};
use oval_syntax::{SyntaxKind, SyntaxNode};

use crate::model::{Ballot, BallotItem, Candidate, Compiled, Counts, Guide, Party, Section};

pub(crate) fn compile(document: &Document) -> Option<Compiled> {
    let candidates = candidates(document.syntax())?;
    let parties = parties(document.syntax(), &candidates)?;
    let mut ballots = Vec::new();
    for root in document.ballots() {
        ballots.push(ballot(&root, &[])?);
    }
    let counts = counts(document.syntax(), &parties, &candidates);
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
        ballots,
        counts,
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

fn ballot(ballot: &ast::Ballot, path: &[String]) -> Option<Ballot> {
    let id = id(ballot.identifier())?;
    let mut inner = path.to_vec();
    inner.push(id.clone());
    let mut items = Vec::new();
    for item in ballot.items() {
        match item {
            ast::BallotItem::Section(section) => items.push(BallotItem::Section(Section {
                title: section.title()?.value().into_owned(),
                note: markdown(section.note()),
            })),
            ast::BallotItem::Ballot(sub_ballot) => {
                items.push(BallotItem::Ballot(self::ballot(&sub_ballot, &inner)?));
            }
            ast::BallotItem::Contest(_) | ast::BallotItem::Measure(_) => {}
        }
    }
    Some(Ballot {
        id,
        title: ballot.name()?.value().into_owned(),
        eligibility: string(ballot.eligibility()),
        note: markdown(ballot.note()),
        path: path.to_vec(),
        items,
    })
}

fn counts(root: &SyntaxNode, parties: &[Party], candidates: &[Candidate]) -> Counts {
    let count = |kind| {
        root.descendants()
            .filter(|node| node.kind() == kind)
            .count()
    };
    Counts {
        ballots: count(SyntaxKind::Ballot),
        sections: count(SyntaxKind::Section),
        contests: count(SyntaxKind::Contest),
        measures: count(SyntaxKind::Measure),
        campaigns: count(SyntaxKind::Campaign),
        candidates: candidates.len(),
        parties: parties.len(),
    }
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

    const BALLOTS: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Dem\"\nballot @state \"State\"\n  eligibility \"Everyone\"\n  note:\n    The state.\n  section \"Offices\"\n    note:\n      Vote.\n    contest @gov \"Governor\" single\n      campaign @a \"A\"\n      campaign party @dem\n      recommend no-position\n  ballot @city \"City\"\n    section \"Measures\"\n      measure @m \"M\"\n        recommend yes\n    ballot @ward \"Ward\"\n      measure @w \"W\"\n        recommend no\n  measure @s \"S\"\n    recommend yes\nballot @county \"County\"\n  contest @sheriff \"Sheriff\" single\n    campaign\n      @b \"B\"\n      @c \"C\"\n    recommend no-position\n";

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

    #[test]
    fn ballots_compile_as_a_tree_with_the_path_to_each() {
        let compiled = compiled(BALLOTS);
        assert_eq!(
            compiled["ballots"],
            json!([
                {
                    "kind": "ballot",
                    "id": "state",
                    "title": "State",
                    "eligibility": "Everyone",
                    "note": "The state.",
                    "path": [],
                    "items": [
                        { "kind": "section", "title": "Offices", "note": "Vote." },
                        {
                            "kind": "ballot",
                            "id": "city",
                            "title": "City",
                            "path": ["state"],
                            "items": [
                                { "kind": "section", "title": "Measures" },
                                {
                                    "kind": "ballot",
                                    "id": "ward",
                                    "title": "Ward",
                                    "path": ["state", "city"],
                                    "items": [],
                                },
                            ],
                        },
                    ],
                },
                { "kind": "ballot", "id": "county", "title": "County", "path": [], "items": [] },
            ])
        );
    }

    #[test]
    fn counts_total_the_whole_document() {
        assert_eq!(
            compiled(BALLOTS)["counts"],
            json!({
                "ballots": 4,
                "sections": 2,
                "contests": 2,
                "measures": 3,
                "campaigns": 3,
                "candidates": 3,
                "parties": 1,
            })
        );
    }
}
