use std::collections::HashMap;

use oval_core::index::Index;
use oval_syntax::ast::{Document, Identifier, Member};

pub(crate) fn display_names(document: &Document, index: &Index) -> HashMap<String, String> {
    let mut names = HashMap::new();
    let mut add = |id: Option<Identifier>, name: Option<String>| {
        if let (Some(id), Some(name)) = (id, name) {
            names.entry(id.name()).or_insert(name);
        }
    };
    for party in document.parties() {
        add(party.id(), party.name());
    }
    for candidate in document.candidates() {
        add(candidate.id(), candidate.name());
    }
    for party in &index.inline_parties {
        add(party.id(), party.name());
    }
    for contest in &index.contests {
        add(contest.id(), contest.title());
        for member in contest
            .campaigns()
            .flat_map(|campaign| campaign.members().collect::<Vec<Member>>())
        {
            if member.is_declaration() {
                add(member.id(), member.name());
            }
        }
    }
    for measure in &index.measures {
        add(measure.id(), measure.title());
    }
    for ballot in &index.ballots {
        add(ballot.id(), ballot.title());
    }
    names
}
