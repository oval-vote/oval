mod ballots;
mod campaigns;
mod declarations;
mod questions;
mod recommendations;

use oval_syntax::ast::{
    AstNode, AstToken, Document, Identifier, MarkdownField, StringField, StringListField, UrlField,
};
use oval_syntax::{SyntaxKind, SyntaxNode};

use crate::model::{Candidate, Compiled, Counts, Guide, Party};

struct Declarations {
    parties: Vec<Party>,
    candidates: Vec<Candidate>,
}

impl Declarations {
    fn party(&self, id: &str) -> Option<&Party> {
        self.parties.iter().find(|party| party.id == id)
    }

    fn candidate(&self, id: &str) -> Option<&Candidate> {
        self.candidates.iter().find(|candidate| candidate.id == id)
    }
}

pub(crate) fn compile(document: &Document) -> Option<Compiled> {
    let candidates = declarations::candidates(document.syntax())?;
    let parties = declarations::parties(document.syntax(), &candidates)?;
    let mut declarations = Declarations {
        parties,
        candidates,
    };
    let mut roots = Vec::new();
    for root in document.ballots() {
        roots.push(ballots::ballot(&root, &[], &declarations)?);
    }
    declarations::contests(&mut declarations, &ballots::contests(&roots));
    let Declarations {
        parties,
        candidates,
    } = declarations;
    let questions = ballots::questions(&roots);
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
        ballots: roots,
        questions,
        counts,
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

fn strings(field: Option<StringListField>) -> Option<Vec<String>> {
    let strings = field?.strings();
    Some(strings.map(|string| string.value().into_owned()).collect())
}

fn url(field: Option<UrlField>) -> Option<String> {
    Some(field?.value()?.text().to_owned())
}

fn markdown(field: Option<MarkdownField>) -> Option<String> {
    Some(field?.body()?.value())
}

#[cfg(test)]
const BALLOTS: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"Dem\"\nballot @state \"State\"\n  eligibility \"Everyone\"\n  note:\n    The state.\n  section \"Offices\"\n    note:\n      Vote.\n    contest @gov \"Governor\" single\n      campaign @a \"A\"\n      campaign party @dem\n      recommend no-position\n  ballot @city \"City\"\n    section \"Measures\"\n      measure @m \"M\"\n        recommend yes\n    ballot @ward \"Ward\"\n      measure @w \"W\"\n        recommend no\n  measure @s \"S\"\n    recommend yes\nballot @county \"County\"\n  contest @sheriff \"Sheriff\" single\n    campaign\n      @b \"B\"\n      @c \"C\"\n    recommend no-position\n";

#[cfg(test)]
fn compiled(text: &str) -> serde_json::Value {
    let compiled = crate::check(text)
        .unwrap()
        .compile()
        .expect("the document compiles");
    serde_json::from_str(&compiled.to_json()).unwrap()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{BALLOTS, compiled};
    use crate::validate::VALID;

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
