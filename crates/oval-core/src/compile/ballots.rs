use oval_syntax::ast;

use super::questions::{contest, measure};
use super::{Declarations, id, markdown, string};
use crate::model::{
    Ballot, BallotItem, Contest, Location, Measure, QuestionEntry, QuestionKind, Section,
    SectionItem,
};

pub(super) fn ballot(
    ballot: &ast::Ballot,
    path: &[String],
    declarations: &Declarations,
) -> Option<Ballot> {
    let id = id(ballot.identifier())?;
    let mut inner = path.to_vec();
    inner.push(id.clone());
    let location = Location {
        ballot: id.clone(),
        section: None,
        path: inner.clone(),
    };
    let mut items = Vec::new();
    for item in ballot.items() {
        items.push(match item {
            ast::BallotItem::Section(section) => {
                BallotItem::Section(self::section(&section, &location, declarations)?)
            }
            ast::BallotItem::Contest(contest) => {
                BallotItem::Contest(self::contest(&contest, location.clone(), declarations)?)
            }
            ast::BallotItem::Measure(measure) => {
                BallotItem::Measure(self::measure(&measure, location.clone())?)
            }
            ast::BallotItem::Ballot(sub_ballot) => {
                BallotItem::Ballot(self::ballot(&sub_ballot, &inner, declarations)?)
            }
        });
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

fn section(
    section: &ast::Section,
    location: &Location,
    declarations: &Declarations,
) -> Option<Section> {
    let title = section.title()?.value().into_owned();
    let location = Location {
        section: Some(title.clone()),
        ..location.clone()
    };
    let mut items = Vec::new();
    for item in section.items() {
        items.push(match item {
            ast::SectionItem::Contest(contest) => {
                SectionItem::Contest(self::contest(&contest, location.clone(), declarations)?)
            }
            ast::SectionItem::Measure(measure) => {
                SectionItem::Measure(self::measure(&measure, location.clone())?)
            }
        });
    }
    Some(Section {
        title,
        note: markdown(section.note()),
        items,
    })
}

enum Question<'a> {
    Contest(&'a Contest),
    Measure(&'a Measure),
}

fn walk<'a>(ballots: &'a [Ballot], questions: &mut Vec<Question<'a>>) {
    for ballot in ballots {
        for item in &ballot.items {
            match item {
                BallotItem::Section(section) => {
                    for item in &section.items {
                        questions.push(match item {
                            SectionItem::Contest(contest) => Question::Contest(contest),
                            SectionItem::Measure(measure) => Question::Measure(measure),
                        });
                    }
                }
                BallotItem::Contest(contest) => questions.push(Question::Contest(contest)),
                BallotItem::Measure(measure) => questions.push(Question::Measure(measure)),
                BallotItem::Ballot(sub_ballot) => walk(std::slice::from_ref(sub_ballot), questions),
            }
        }
    }
}

fn in_ballot_order(ballots: &[Ballot]) -> Vec<Question<'_>> {
    let mut questions = Vec::new();
    walk(ballots, &mut questions);
    questions
}

pub(super) fn contests(ballots: &[Ballot]) -> Vec<&Contest> {
    in_ballot_order(ballots)
        .into_iter()
        .filter_map(|question| match question {
            Question::Contest(contest) => Some(contest),
            Question::Measure(_) => None,
        })
        .collect()
}

pub(super) fn questions(ballots: &[Ballot]) -> Vec<QuestionEntry> {
    in_ballot_order(ballots)
        .into_iter()
        .map(|question| match question {
            Question::Contest(contest) => QuestionEntry {
                kind: QuestionKind::Contest,
                id: contest.id.clone(),
                title: contest.title.clone(),
                location: contest.location.clone(),
                required: contest.required,
            },
            Question::Measure(measure) => QuestionEntry {
                kind: QuestionKind::Measure,
                id: measure.id.clone(),
                title: measure.title.clone(),
                location: measure.location.clone(),
                required: measure.required,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::compile::{BALLOTS, compiled};

    fn outline(item: &Value) -> String {
        let kind = item["kind"].as_str().unwrap();
        let name = item
            .get("id")
            .or_else(|| item.get("title"))
            .and_then(Value::as_str)
            .unwrap();
        match item.get("items").and_then(Value::as_array) {
            Some(items) => {
                let items: Vec<String> = items.iter().map(outline).collect();
                format!("{kind} {name} [{}]", items.join(", "))
            }
            None => format!("{kind} {name}"),
        }
    }

    fn without_items(item: &Value) -> Value {
        let mut item = item.clone();
        item.as_object_mut().unwrap().remove("items");
        item
    }

    #[test]
    fn ballots_compile_as_a_tree_in_ballot_order() {
        let ballots = &compiled(BALLOTS)["ballots"];
        let roots: Vec<String> = ballots.as_array().unwrap().iter().map(outline).collect();
        assert_eq!(
            roots,
            [
                "ballot state [section Offices [contest gov], ballot city [section Measures \
                 [measure m], ballot ward [measure w]], measure s]",
                "ballot county [contest sheriff]",
            ]
        );
    }

    #[test]
    fn each_ballot_has_its_fields_and_the_path_to_it() {
        let state = &compiled(BALLOTS)["ballots"][0];
        let city = &state["items"][1];
        assert_eq!(
            without_items(state),
            json!({
                "kind": "ballot",
                "id": "state",
                "title": "State",
                "eligibility": "Everyone",
                "note": "The state.",
                "path": [],
            })
        );
        assert_eq!(
            without_items(&state["items"][0]),
            json!({ "kind": "section", "title": "Offices", "note": "Vote." })
        );
        assert_eq!(
            without_items(city),
            json!({ "kind": "ballot", "id": "city", "title": "City", "path": ["state"] })
        );
        assert_eq!(city["items"][1]["path"], json!(["state", "city"]));
    }

    #[test]
    fn questions_list_every_contest_and_measure_in_ballot_order_with_its_location() {
        let compiled = compiled(BALLOTS);
        assert_eq!(
            compiled["questions"],
            json!([
                {
                    "kind": "contest",
                    "id": "gov",
                    "title": "Governor",
                    "location": { "ballot": "state", "section": "Offices", "path": ["state"] },
                    "required": false,
                },
                {
                    "kind": "measure",
                    "id": "m",
                    "title": "M",
                    "location": {
                        "ballot": "city",
                        "section": "Measures",
                        "path": ["state", "city"],
                    },
                    "required": false,
                },
                {
                    "kind": "measure",
                    "id": "w",
                    "title": "W",
                    "location": { "ballot": "ward", "path": ["state", "city", "ward"] },
                    "required": false,
                },
                {
                    "kind": "measure",
                    "id": "s",
                    "title": "S",
                    "location": { "ballot": "state", "path": ["state"] },
                    "required": false,
                },
                {
                    "kind": "contest",
                    "id": "sheriff",
                    "title": "Sheriff",
                    "location": { "ballot": "county", "path": ["county"] },
                    "required": false,
                },
            ])
        );
        let governor = &compiled["ballots"][0]["items"][0]["items"][0];
        assert_eq!(governor["location"], compiled["questions"][0]["location"]);
    }
}
