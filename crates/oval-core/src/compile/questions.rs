use oval_syntax::SyntaxNode;
use oval_syntax::ast::{self, AstNode, Extensibility, IntegerLiteral, Position, Requirement};

use super::campaigns::campaigns;
use super::recommendations::recommendation;
use super::{Declarations, id, markdown, string, strings};
use crate::model::{Contest, Location, Measure, MeasurePosition, MeasureRecommendation, Method};
use crate::validate::nearest;

pub(super) fn contest(
    contest: &ast::Contest,
    location: Location,
    declarations: &Declarations,
) -> Option<Contest> {
    let method = method(contest)?;
    let mut campaigns = campaigns(contest, declarations)?;
    let recommendation = recommendation(contest, &mut campaigns)?;
    let extensible = method != Method::Retention
        && nearest::<Extensibility>(contest.syntax())
            .is_some_and(|extensibility| extensibility.is_extensible());
    Some(Contest {
        id: id(contest.identifier())?,
        title: contest.name()?.value().into_owned(),
        subtitle: string(contest.subtitle()),
        method,
        tabulation: string(contest.tabulation()),
        note: markdown(contest.note()),
        location,
        required: required(contest.syntax()),
        extensible,
        campaigns,
        recommendation,
    })
}

fn method(contest: &ast::Contest) -> Option<Method> {
    let kind = contest.kind()?;
    let method = match kind.method() {
        ast::Method::Single => Method::Single {
            seats: number(kind.seats(), 1)?,
        },
        ast::Method::Multiple => {
            let votes = kind.votes()?.value()?;
            Method::Multiple {
                votes,
                seats: number(kind.seats(), votes)?,
                min: number(kind.min(), 1)?,
            }
        }
        ast::Method::Ranked => {
            let complete = kind.all().is_some();
            let min = if complete {
                u64::try_from(contest.campaigns().count()).ok()?
            } else {
                number(kind.min(), 1)?
            };
            let max = match kind.max() {
                Some(max) => Some(max.value()?),
                None => None,
            };
            Method::Ranked {
                seats: number(kind.seats(), 1)?,
                max,
                min,
                complete,
            }
        }
        ast::Method::Retention => Method::Retention,
    };
    Some(method)
}

fn number(integer: Option<IntegerLiteral>, default: u64) -> Option<u64> {
    match integer {
        Some(integer) => integer.value(),
        None => Some(default),
    }
}

pub(super) fn measure(measure: &ast::Measure, location: Location) -> Option<Measure> {
    let recommendation = measure.recommendations().next()?;
    let position = match recommendation.position()?.value() {
        Position::Yes => MeasurePosition::Yes,
        Position::No => MeasurePosition::No,
        Position::NoPosition => MeasurePosition::NoPosition,
        Position::Abstain => MeasurePosition::Abstain,
    };
    Some(Measure {
        id: id(measure.identifier())?,
        title: measure.name()?.value().into_owned(),
        subtitle: string(measure.subtitle()),
        description: markdown(measure.description()),
        if_yes: markdown(measure.if_yes()),
        if_no: markdown(measure.if_no()),
        placement: string(measure.placement()),
        tabulation: string(measure.tabulation()),
        fiscal_impact: string(measure.fiscal_impact()),
        supporters: strings(measure.supporters()),
        opponents: strings(measure.opponents()),
        note: markdown(measure.note()),
        location,
        required: required(measure.syntax()),
        recommendation: MeasureRecommendation {
            position,
            justification: markdown(recommendation.justification()),
            note: markdown(recommendation.note()),
        },
    })
}

fn required(question: &SyntaxNode) -> bool {
    nearest::<Requirement>(question).is_some_and(|requirement| requirement.is_required())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::compile::compiled;

    fn method(kind: &str) -> Value {
        let campaigns = if kind == "retention" {
            "    campaign @a \"A\"\n"
        } else {
            "    campaign @a \"A\"\n    campaign @b \"B\"\n    campaign @c \"C\"\n"
        };
        let text = format!(
            "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @z \"Z\"\n  contest @k \"K\" {kind}\n{campaigns}    recommend no-position\n"
        );
        compiled(&text)["ballots"][0]["items"][0]["method"].clone()
    }

    #[test]
    fn a_method_has_its_numbers_with_the_defaults_filled_in() {
        assert_eq!(method("single"), json!({ "kind": "single", "seats": 1 }));
        assert_eq!(
            method("single seats 2"),
            json!({ "kind": "single", "seats": 2 })
        );
        assert_eq!(
            method("multiple 3"),
            json!({ "kind": "multiple", "votes": 3, "seats": 3, "min": 1 })
        );
        assert_eq!(
            method("multiple 3 seats 04 min 2"),
            json!({ "kind": "multiple", "votes": 3, "seats": 4, "min": 2 })
        );
        assert_eq!(
            method("ranked"),
            json!({ "kind": "ranked", "seats": 1, "min": 1, "complete": false })
        );
        assert_eq!(
            method("ranked min 2 seats 3 max 6"),
            json!({ "kind": "ranked", "seats": 3, "max": 6, "min": 2, "complete": false })
        );
        assert_eq!(
            method("ranked all seats 2"),
            json!({ "kind": "ranked", "seats": 2, "min": 3, "complete": true })
        );
        assert_eq!(method("retention"), json!({ "kind": "retention" }));
    }

    #[test]
    fn a_question_takes_the_nearest_requirement_and_a_contest_the_nearest_extensibility() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nrequired extensible ballot @b \"B\"\n  contest @k1 \"K1\" single\n    campaign @a1 \"A\"\n    recommend no-position\n  optional exhaustive section \"S\"\n    contest @k2 \"K2\" single\n      campaign @a2 \"A\"\n      recommend no-position\n    required contest @k3 \"K3\" single\n      campaign @a3 \"A\"\n      recommend no-position\n    extensible contest @k4 \"K4\" single\n      campaign @a4 \"A\"\n      recommend no-position\n    measure @m1 \"M1\"\n      recommend yes\n  contest @r \"R\" retention\n    campaign @u \"U\"\n    recommend no-position\n  ballot @sub \"Sub\"\n    optional measure @m2 \"M2\"\n      recommend yes\n    measure @m3 \"M3\"\n      recommend yes\nballot @c \"C\"\n  contest @k5 \"K5\" single\n    campaign @a5 \"A\"\n    recommend no-position\n";
        let compiled = compiled(text);
        let required: Vec<(&str, bool)> = compiled["questions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|question| {
                (
                    question["id"].as_str().unwrap(),
                    question["required"].as_bool().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            required,
            [
                ("k1", true),
                ("k2", false),
                ("k3", true),
                ("k4", false),
                ("m1", false),
                ("r", true),
                ("m2", false),
                ("m3", true),
                ("k5", false),
            ]
        );
        let b = &compiled["ballots"][0]["items"];
        let section = &b[1]["items"];
        let k5 = &compiled["ballots"][1]["items"][0];
        let extensible: Vec<&Value> = [&b[0], &section[0], &section[1], &section[2], &b[2], k5]
            .iter()
            .map(|contest| &contest["extensible"])
            .collect();
        assert_eq!(
            extensible,
            [true, false, false, true, false, false]
                .map(Value::Bool)
                .iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_measure_has_its_fields_and_recommendation() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  measure @prop1 \"Proposition 1\"\n    subtitle \"Housing Bond\"\n    description:\n      Authorizes bonds.\n    if_yes:\n      Bonds are sold.\n    if_no:\n      No bonds.\n    placement \"Legislative referral\"\n    tabulation \"Simple majority\"\n    fiscal_impact \"$140 million a year\"\n    supporters \"Housing Coalition\", \"Labor Federation\"\n    opponents none\n    note:\n      Big.\n    recommend no\n      justification:\n        Too costly.\n      note:\n        Close call.\n  measure @m \"M\"\n    recommend no-position\n  optional measure @o \"O\"\n    recommend abstain\n  measure @y \"Y\"\n    recommend yes\n";
        let compiled = compiled(text);
        let items = &compiled["ballots"][0]["items"];
        let location = json!({ "ballot": "b", "path": ["b"] });
        assert_eq!(
            items[0],
            json!({
                "kind": "measure",
                "id": "prop1",
                "title": "Proposition 1",
                "subtitle": "Housing Bond",
                "description": "Authorizes bonds.",
                "if_yes": "Bonds are sold.",
                "if_no": "No bonds.",
                "placement": "Legislative referral",
                "tabulation": "Simple majority",
                "fiscal_impact": "$140 million a year",
                "supporters": ["Housing Coalition", "Labor Federation"],
                "opponents": [],
                "note": "Big.",
                "location": location,
                "required": false,
                "recommendation": {
                    "position": "no",
                    "justification": "Too costly.",
                    "note": "Close call.",
                },
            })
        );
        assert_eq!(
            items[1],
            json!({
                "kind": "measure",
                "id": "m",
                "title": "M",
                "location": location,
                "required": false,
                "recommendation": { "position": "no-position" },
            })
        );
        assert_eq!(items[2]["recommendation"], json!({ "position": "abstain" }));
        assert_eq!(items[3]["recommendation"], json!({ "position": "yes" }));
    }

    #[test]
    fn a_contest_has_its_fields() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @mayor \"Mayor\" ranked max 5\n    subtitle \"Rank up to five\"\n    tabulation \"Instant runoff\"\n    note:\n      Open seat.\n    campaign @a \"A\"\n    recommend no-position\n";
        let contest = compiled(text)["ballots"][0]["items"][0].clone();
        for (key, value) in [
            ("kind", json!("contest")),
            ("id", json!("mayor")),
            ("title", json!("Mayor")),
            ("subtitle", json!("Rank up to five")),
            ("tabulation", json!("Instant runoff")),
            ("note", json!("Open seat.")),
            ("location", json!({ "ballot": "b", "path": ["b"] })),
            ("required", json!(false)),
            ("extensible", json!(false)),
        ] {
            assert_eq!(contest[key], value, "{key}");
        }
    }
}
