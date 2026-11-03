use super::{AstNode, Date, Document, markdown_value, string_value};
use crate::Keyword;
use crate::parser::parse;

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
fn missing_fields_are_none_not_panics() {
    let doc = document("date 2026-11-03\n");
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
