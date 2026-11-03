use super::{
    AstNode, BallotItem, ContestKind, Date, Document, Position, SectionItem, markdown_value,
    string_value,
};
use crate::parser::parse;
use crate::{Keyword, SpecVersion};

fn document(source: &str) -> Document {
    let result = parse(source);
    Document::cast(result.syntax()).expect("root is a Document")
}

#[test]
fn guide_fields_from_the_spec_example() {
    let doc = document(
        "guide \"Franklin Voter Guide\"\nelection \"November 3, 2026 General Election\"\ndate 2026-11-03\n\nnote:\n  Written by the Franklin Civic Alliance.\n",
    );
    assert_eq!(
        doc.guide().and_then(|f| f.value()).as_deref(),
        Some("Franklin Voter Guide")
    );
    assert_eq!(
        doc.election().and_then(|f| f.value()).as_deref(),
        Some("November 3, 2026 General Election")
    );
    assert_eq!(
        doc.date().and_then(|f| f.value()),
        Some(Date {
            year: 2026,
            month: 11,
            day: 3
        })
    );
    assert_eq!(
        doc.note().and_then(|f| f.value()).as_deref(),
        Some("Written by the Franklin Civic Alliance.")
    );
    assert_eq!(doc.guide().and_then(|f| f.keyword()), Some(Keyword::Guide));
}

#[test]
fn the_version_marker_is_read_as_a_spec_version() {
    let doc = document("oval 0.1\nguide \"G\"\n");
    assert_eq!(
        doc.version().and_then(|m| m.value()),
        Some(SpecVersion { major: 0, minor: 1 })
    );
    assert_eq!(
        document("oval 12.30\n").version().and_then(|m| m.value()),
        Some(SpecVersion {
            major: 12,
            minor: 30
        })
    );
}

#[test]
fn a_marker_without_a_version_is_present_but_empty() {
    let doc = document("oval\n");
    let marker = doc.version().expect("marker node exists");
    assert_eq!(marker.value(), None);
}

#[test]
fn missing_fields_are_none_not_panics() {
    let doc = document("date 2026-11-03\n");
    assert_eq!(doc.version(), None);
    assert_eq!(doc.guide(), None);
    assert_eq!(doc.election(), None);
    assert_eq!(doc.note(), None);
    assert!(doc.date().is_some());
    assert_eq!(doc.parties().count(), 0);
    assert_eq!(doc.candidates().count(), 0);
}

#[test]
fn a_field_with_a_missing_value_is_present_but_empty() {
    let doc = document("guide\n");
    let field = doc.guide().expect("field node exists");
    assert_eq!(field.value(), None);
    assert_eq!(field.keyword(), Some(Keyword::Guide));
}

#[test]
fn parties_and_their_fields() {
    let doc = document(
        "party @dem \"Democratic\"\n  abbreviation \"D\"\n  website https://dem.example\n  note:\n    Big tent.\nparty @gop \"Republican\"\n",
    );
    let parties: Vec<_> = doc.parties().collect();
    assert_eq!(parties.len(), 2);
    let dem = &parties[0];
    assert_eq!(
        dem.id().map(|i| i.name().to_owned()).as_deref(),
        Some("dem")
    );
    assert_eq!(dem.name().as_deref(), Some("Democratic"));
    assert_eq!(
        dem.abbreviation().and_then(|f| f.value()).as_deref(),
        Some("D")
    );
    assert_eq!(
        dem.website().and_then(|f| f.value()).as_deref(),
        Some("https://dem.example")
    );
    assert_eq!(
        dem.note().and_then(|f| f.value()).as_deref(),
        Some("Big tent.")
    );
    let gop = &parties[1];
    assert_eq!(gop.abbreviation(), None);
    assert_eq!(gop.website(), None);
    assert_eq!(gop.id().map(|i| i.range().len()), Some(4.into()));
}

#[test]
fn candidates_with_a_reference_and_an_inline_party() {
    let doc = document(
        "candidate @nreyes \"Nadia Reyes\"\n  designation \"United States Senator\"\n  party @dem\n  note:\n    Appointed in 2025.\ncandidate @lmoore \"Lena Moore\"\n  party @green \"Green\"\n    abbreviation \"G\"\n",
    );
    let candidates: Vec<_> = doc.candidates().collect();
    assert_eq!(candidates.len(), 2);

    let reyes = &candidates[0];
    assert_eq!(
        reyes.id().map(|i| i.name().to_owned()).as_deref(),
        Some("nreyes")
    );
    assert_eq!(reyes.name().as_deref(), Some("Nadia Reyes"));
    assert_eq!(
        reyes.designation().and_then(|f| f.value()).as_deref(),
        Some("United States Senator")
    );
    let party = reyes.party().expect("party line");
    assert_eq!(
        party.id().map(|i| i.name().to_owned()).as_deref(),
        Some("dem")
    );
    assert!(!party.is_declaration());
    assert_eq!(party.name(), None);
    assert_eq!(
        reyes.note().and_then(|f| f.value()).as_deref(),
        Some("Appointed in 2025.")
    );

    let moore = &candidates[1];
    assert_eq!(moore.designation(), None);
    let party = moore.party().expect("party line");
    assert!(party.is_declaration());
    assert_eq!(party.name().as_deref(), Some("Green"));
    assert_eq!(
        party.abbreviation().and_then(|f| f.value()).as_deref(),
        Some("G")
    );
}

#[test]
fn a_declaration_missing_its_identifier_still_has_a_name() {
    let doc = document("party \"D\"\n");
    let party = doc.parties().next().expect("party node");
    assert_eq!(party.id(), None);
    assert_eq!(party.name().as_deref(), Some("D"));
}

#[test]
fn string_values_resolve_exactly_two_escapes() {
    assert_eq!(string_value("\"plain\""), "plain");
    assert_eq!(
        string_value("\"a \\\"quoted\\\" word\""),
        "a \"quoted\" word"
    );
    assert_eq!(string_value("\"back\\\\slash\""), "back\\slash");
    assert_eq!(string_value("\"unknown \\n stays\""), "unknown \\n stays");
    assert_eq!(string_value("\"\""), "");
    assert_eq!(string_value("\"José\""), "José");
}

#[test]
fn markdown_values_strip_common_indentation() {
    assert_eq!(markdown_value("  Hello.\n"), "Hello.");
    assert_eq!(
        markdown_value("  First.\n\n  Second.\n"),
        "First.\n\nSecond."
    );
    assert_eq!(markdown_value("      deep\n    less\n"), "  deep\nless");
    assert_eq!(markdown_value("  a\n   \n  b\n"), "a\n\nb");
    assert_eq!(markdown_value("  crlf\r\n  lines\r\n"), "crlf\nlines");
    assert_eq!(markdown_value("  no terminator"), "no terminator");
    assert_eq!(
        markdown_value("  - item\n    continued\n  - next\n"),
        "- item\n  continued\n- next"
    );
}

#[test]
fn date_values_are_split_as_written() {
    let doc = document("date 2026-13-45\n");
    assert_eq!(
        doc.date().and_then(|f| f.value()),
        Some(Date {
            year: 2026,
            month: 13,
            day: 45
        })
    );
}

const FIXTURE: &str = include_str!("../../tests/fixtures/franklin.oval");

#[test]
fn ballots_sections_and_items() {
    let doc = document(
        "ballot @franklin \"State of Franklin\"\n  eligibility \"Everyone\"\n  section \"Offices\"\n    note:\n      Top.\n    contest @gov \"Governor\" single\n      campaign @a \"A\"\n      recommend\n        support @a\n    measure @m \"M\"\n      recommend yes\n  ballot @city \"City\"\n    measure @c \"C\"\n      recommend no\n",
    );
    let roots: Vec<_> = doc.ballots().collect();
    assert_eq!(roots.len(), 1);
    let root = &roots[0];
    assert_eq!(
        root.id().map(|i| i.name().to_owned()).as_deref(),
        Some("franklin")
    );
    assert_eq!(root.title().as_deref(), Some("State of Franklin"));
    assert_eq!(
        root.eligibility().and_then(|f| f.value()).as_deref(),
        Some("Everyone")
    );
    let items: Vec<_> = root.items().collect();
    assert_eq!(items.len(), 2);
    let BallotItem::Section(section) = &items[0] else {
        panic!("section first")
    };
    let BallotItem::Ballot(sub) = &items[1] else {
        panic!("sub-ballot second")
    };
    assert_eq!(section.title().as_deref(), Some("Offices"));
    assert_eq!(
        section.note().and_then(|f| f.value()).as_deref(),
        Some("Top.")
    );
    let section_items: Vec<_> = section.items().collect();
    assert!(
        matches!(&section_items[0], SectionItem::Contest(c) if c.title().as_deref() == Some("Governor"))
    );
    assert!(
        matches!(&section_items[1], SectionItem::Measure(m) if m.title().as_deref() == Some("M"))
    );
    assert_eq!(sub.title().as_deref(), Some("City"));
    assert!(matches!(sub.items().next(), Some(BallotItem::Measure(_))));
}

fn indent(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        out.push_str("  ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn first_contest(source: &str) -> super::Contest {
    let doc = document(&format!("ballot @b \"B\"\n{}", indent(source)));
    let Some(BallotItem::Contest(contest)) = doc.ballots().next().and_then(|b| b.items().next())
    else {
        panic!("a contest")
    };
    contest
}

#[test]
fn contest_kinds() {
    let cases = [
        ("single", ContestKind::Single),
        ("retention", ContestKind::Retention),
        ("multiple 3", ContestKind::Multiple { seats: Some(3) }),
        ("multiple", ContestKind::Multiple { seats: None }),
        ("ranked", ContestKind::Ranked { max: None }),
        ("ranked max 10", ContestKind::Ranked { max: Some(10) }),
    ];
    for (text, expected) in cases {
        let contest = first_contest(&format!("contest @c \"C\" {text}\n  note:\n    x\n"));
        assert_eq!(contest.kind(), Some(expected), "{text}");
    }
    let contest = first_contest("contest @c \"C\"\n  note:\n    x\n");
    assert_eq!(contest.kind(), None);
}

#[test]
fn contest_fields_campaigns_and_recommendation() {
    let contest = first_contest(
        "contest @mayor \"Mayor\" ranked max 5\n  subtitle \"Rank five\"\n  tabulation \"IRV\"\n  campaign incumbent @b \"Bea\"\n    designation \"Mayor\"\n    website https://bea.example\n  campaign withdrawn @c \"Cas\"\n  recommend\n    support @a\n    support @b @d\n    oppose @c\n    justification:\n      Why.\n    note:\n      Also.\n",
    );
    assert_eq!(
        contest.id().map(|i| i.name().to_owned()).as_deref(),
        Some("mayor")
    );
    assert_eq!(
        contest.subtitle().and_then(|f| f.value()).as_deref(),
        Some("Rank five")
    );
    assert_eq!(
        contest.tabulation().and_then(|f| f.value()).as_deref(),
        Some("IRV")
    );
    let campaigns: Vec<_> = contest.campaigns().collect();
    assert_eq!(campaigns.len(), 2);
    assert!(campaigns[0].is_incumbent() && !campaigns[0].is_withdrawn());
    assert!(!campaigns[1].is_incumbent() && campaigns[1].is_withdrawn());
    assert_eq!(
        campaigns[0]
            .designation()
            .and_then(|f| f.value())
            .as_deref(),
        Some("Mayor")
    );
    assert_eq!(
        campaigns[0].website().and_then(|f| f.value()).as_deref(),
        Some("https://bea.example")
    );
    let members: Vec<_> = campaigns[0].members().collect();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].name().as_deref(), Some("Bea"));
    assert!(members[0].is_declaration());
    assert_eq!(members[0].role(), None);

    let recs: Vec<_> = contest.recommendations().collect();
    assert_eq!(recs.len(), 1);
    let rec = &recs[0];
    assert!(!rec.is_no_position());
    let tiers: Vec<Vec<String>> = rec
        .support()
        .map(|t| t.candidates().map(|c| c.name().to_owned()).collect())
        .collect();
    assert_eq!(
        tiers,
        [vec!["a".to_owned()], vec!["b".to_owned(), "d".to_owned()]]
    );
    let opposed: Vec<String> = rec
        .oppose()
        .flat_map(|o| {
            o.candidates()
                .map(|c| c.name().to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(opposed, ["c"]);
    assert_eq!(
        rec.justification().and_then(|f| f.value()).as_deref(),
        Some("Why.")
    );
    assert_eq!(rec.note().and_then(|f| f.value()).as_deref(), Some("Also."));
}

#[test]
fn ticket_members_in_block_form() {
    let contest = first_contest(
        "contest @p \"President\" single\n  campaign incumbent\n    @j \"Jordan Lee\" as \"Governor\"\n      designation \"Governor\"\n      party @dem\n    @m \"Maya Patel\" as \"Lieutenant Governor\"\n    website https://t.example\n  campaign @r\n  recommend no-position\n",
    );
    let campaigns: Vec<_> = contest.campaigns().collect();
    let members: Vec<_> = campaigns[0].members().collect();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].name().as_deref(), Some("Jordan Lee"));
    assert_eq!(members[0].role().as_deref(), Some("Governor"));
    assert_eq!(
        members[0].designation().and_then(|f| f.value()).as_deref(),
        Some("Governor")
    );
    assert_eq!(
        members[0]
            .party()
            .and_then(|p| p.id())
            .map(|i| i.name().to_owned())
            .as_deref(),
        Some("dem")
    );
    assert_eq!(members[1].role().as_deref(), Some("Lieutenant Governor"));
    assert_eq!(
        campaigns[0].website().and_then(|f| f.value()).as_deref(),
        Some("https://t.example")
    );
    let reference: Vec<_> = campaigns[1].members().collect();
    assert_eq!(reference.len(), 1);
    assert!(!reference[0].is_declaration());
    assert_eq!(
        reference[0].id().map(|i| i.name().to_owned()).as_deref(),
        Some("r")
    );
    assert!(contest.recommendations().next().unwrap().is_no_position());
}

fn first_measure(source: &str) -> super::Measure {
    let doc = document(&format!("ballot @b \"B\"\n{}", indent(source)));
    let Some(BallotItem::Measure(measure)) = doc.ballots().next().and_then(|b| b.items().next())
    else {
        panic!("a measure")
    };
    measure
}

#[test]
fn measure_fields_and_positions() {
    let measure = first_measure(
        "measure @p1 \"Proposition 1\"\n  subtitle \"Bond\"\n  description:\n    Text.\n  if_yes:\n    Yes.\n  if_no:\n    No.\n  placement \"Referral\"\n  tabulation \"Majority\"\n  fiscal_impact \"$1\"\n  supporters \"A\", \"B\"\n  opponents none\n  note:\n    N.\n  recommend yes\n    justification:\n      J.\n",
    );
    assert_eq!(
        measure.id().map(|i| i.name().to_owned()).as_deref(),
        Some("p1")
    );
    assert_eq!(measure.title().as_deref(), Some("Proposition 1"));
    assert_eq!(
        measure.subtitle().and_then(|f| f.value()).as_deref(),
        Some("Bond")
    );
    assert_eq!(
        measure.description().and_then(|f| f.value()).as_deref(),
        Some("Text.")
    );
    assert_eq!(
        measure.if_yes().and_then(|f| f.value()).as_deref(),
        Some("Yes.")
    );
    assert_eq!(
        measure.if_no().and_then(|f| f.value()).as_deref(),
        Some("No.")
    );
    assert_eq!(
        measure.placement().and_then(|f| f.value()).as_deref(),
        Some("Referral")
    );
    assert_eq!(
        measure.tabulation().and_then(|f| f.value()).as_deref(),
        Some("Majority")
    );
    assert_eq!(
        measure.fiscal_impact().and_then(|f| f.value()).as_deref(),
        Some("$1")
    );
    assert_eq!(
        measure.supporters().and_then(|f| f.value()),
        Some(vec!["A".to_owned(), "B".to_owned()])
    );
    assert_eq!(measure.opponents().and_then(|f| f.value()), Some(vec![]));
    assert_eq!(
        measure.note().and_then(|f| f.value()).as_deref(),
        Some("N.")
    );
    let rec = measure.recommendations().next().expect("recommendation");
    assert_eq!(rec.position(), Some(Position::Yes));
    assert_eq!(
        rec.justification().and_then(|f| f.value()).as_deref(),
        Some("J.")
    );

    for (text, expected) in [("no", Position::No), ("no-position", Position::NoPosition)] {
        let measure = first_measure(&format!("measure @m \"M\"\n  recommend {text}\n"));
        assert_eq!(
            measure.recommendations().next().unwrap().position(),
            Some(expected)
        );
    }
    let measure = first_measure("measure @m \"M\"\n  recommend\n");
    assert_eq!(measure.recommendations().next().unwrap().position(), None);
    assert_eq!(measure.supporters(), None);
}

#[test]
fn string_list_three_states() {
    let measure =
        first_measure("measure @m \"M\"\n  supporters\n  opponents none\n  recommend yes\n");
    assert_eq!(measure.supporters().and_then(|f| f.value()), None);
    assert_eq!(measure.opponents().and_then(|f| f.value()), Some(vec![]));
    let measure = first_measure("measure @m \"M\"\n  recommend yes\n");
    assert_eq!(measure.supporters(), None);
}

fn walk(
    ballot: &super::Ballot,
    contests: &mut Vec<super::Contest>,
    measures: &mut Vec<super::Measure>,
) {
    for item in ballot.items() {
        match item {
            BallotItem::Contest(c) => contests.push(c),
            BallotItem::Measure(m) => measures.push(m),
            BallotItem::Ballot(b) => walk(&b, contests, measures),
            BallotItem::Section(s) => {
                for item in s.items() {
                    match item {
                        SectionItem::Contest(c) => contests.push(c),
                        SectionItem::Measure(m) => measures.push(m),
                    }
                }
            }
        }
    }
}

#[test]
fn the_fixture_walks_cleanly() {
    let doc = document(FIXTURE);
    assert_eq!(doc.parties().count(), 5);
    assert_eq!(doc.candidates().count(), 2);
    let roots: Vec<_> = doc.ballots().collect();
    assert_eq!(roots.len(), 6);

    let mut contests = Vec::new();
    let mut measures = Vec::new();
    for root in &roots {
        assert!(root.id().is_some() && root.title().is_some());
        walk(root, &mut contests, &mut measures);
    }
    assert_eq!(contests.len(), 44);
    assert_eq!(measures.len(), 28);

    for contest in &contests {
        assert!(contest.id().is_some() && contest.title().is_some() && contest.kind().is_some());
        assert!(contest.campaigns().count() >= 1, "{:?}", contest.title());
        assert_eq!(
            contest.recommendations().count(),
            1,
            "{:?}",
            contest.title()
        );
        for campaign in contest.campaigns() {
            let members: Vec<_> = campaign.members().collect();
            assert_ne!(members.len(), 0);
            assert!(members.iter().all(|m| m.id().is_some()));
        }
    }
    for measure in &measures {
        assert!(measure.id().is_some() && measure.title().is_some());
        let recs: Vec<_> = measure.recommendations().collect();
        assert_eq!(recs.len(), 1);
        assert!(recs[0].position().is_some());
    }
    let tickets = contests
        .iter()
        .flat_map(super::Contest::campaigns)
        .filter(|c| c.members().count() > 1)
        .count();
    assert_eq!(tickets, 6);
    let withdrawn = contests
        .iter()
        .flat_map(super::Contest::campaigns)
        .filter(super::Campaign::is_withdrawn)
        .count();
    assert_eq!(withdrawn, 1);
}
