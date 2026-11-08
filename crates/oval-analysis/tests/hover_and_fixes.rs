//! Tests of hover text and quick fixes over small documents.

use oval_analysis::{fixes, hover};
use oval_syntax::TextSize;

const GUIDE: &str = r#"oval 1
guide "G"
election "E"
date 2026-11-03
party @dem "Democratic"
party @gop "Republican"
candidate @delgado "Rosa Delgado"
  party @dem
  note:
    See [the site](http://delgado.example).
ballot @city "City"
  contest @attorney "City Attorney" single
    campaign @delgado
      website http://delgado.example
    campaign @webb "Marcus Webb"
    recommend
      support @delgato
  measure @a "Proposition A"
"#;

fn at(source: &str, needle: &str) -> TextSize {
    TextSize::try_from(source.find(needle).expect("needle") + 1).expect("fits")
}

fn apply(source: &str, fix: &oval_analysis::Fix) -> String {
    let mut text = source.to_owned();
    for edit in fix.edits.iter().rev() {
        text.replace_range(std::ops::Range::<usize>::from(edit.range), &edit.text);
    }
    text
}

#[test]
fn hovering_an_identifier_names_what_it_declares() {
    let found = hover(GUIDE, at(GUIDE, "@delgado\n")).expect("hover");
    assert_eq!(&GUIDE[found.range], "@delgado");
    assert_eq!(
        found.contents,
        "**candidate** `@delgado`\n\nRosa Delgado\n\nDemocratic\n\nReferenced once"
    );
    let found = hover(GUIDE, at(GUIDE, "@attorney")).expect("hover");
    assert_eq!(
        found.contents,
        "**contest** `@attorney`\n\nCity Attorney\n\nVote for one\n\nNot referenced"
    );
    let found = hover(GUIDE, at(GUIDE, "@gop")).expect("hover");
    assert!(found.contents.starts_with("**party** `@gop`\n\nRepublican"));
}

#[test]
fn hovering_a_keyword_explains_it_and_links_its_section() {
    let found = hover(GUIDE, at(GUIDE, "contest @attorney")).expect("hover");
    assert_eq!(&GUIDE[found.range], "contest");
    assert!(found.contents.starts_with("**keyword** `contest`\n\n"));
    assert!(
        found
            .contents
            .ends_with("[Contest](https://spec.oval.vote/draft/#sec-Contest)")
    );
    let found = hover(GUIDE, at(GUIDE, "recommend\n")).expect("hover");
    assert!(found.contents.contains("#sec-Contest-Recommendation"));
}

#[test]
fn hovering_elsewhere_gives_nothing() {
    assert_eq!(hover(GUIDE, at(GUIDE, "\"City\"")), None);
    assert_eq!(hover(GUIDE, at(GUIDE, "@delgato")), None);
}

#[test]
fn an_insecure_url_is_made_secure_in_fields_and_bodies() {
    let found: Vec<_> = fixes(GUIDE)
        .into_iter()
        .filter(|fix| fix.code == "insecure-url")
        .collect();
    assert_eq!(found.len(), 2);
    for fix in &found {
        assert_eq!(fix.title, "Use https");
        let fixed = apply(GUIDE, fix);
        assert_eq!(fixed.matches("http://").count(), 1);
        assert_eq!(fixed.matches("https://").count(), 1);
    }
}

#[test]
fn a_misspelled_reference_is_changed_to_the_nearest_declaration() {
    let fix = fixes(GUIDE)
        .into_iter()
        .find(|fix| fix.code == "undeclared-reference")
        .expect("fix");
    assert_eq!(fix.title, "Change to `@delgado`");
    assert_eq!(&GUIDE[fix.range], "@delgato");
    assert!(apply(GUIDE, &fix).contains("support @delgado"));
}

#[test]
fn a_missing_guide_field_is_added_after_the_marker() {
    let source = "oval 1\nguide \"G\"\ndate 2026-11-03\n";
    let fix = fixes(source)
        .into_iter()
        .find(|fix| fix.code == "missing-guide-field")
        .expect("fix");
    assert_eq!(fix.title, "Add `election \"\"`");
    assert_eq!(
        apply(source, &fix),
        "oval 1\nelection \"\"\nguide \"G\"\ndate 2026-11-03\n"
    );
}

#[test]
fn a_missing_recommendation_is_added_at_the_end_of_the_question() {
    let fix = fixes(GUIDE)
        .into_iter()
        .find(|fix| fix.code == "missing-recommendation")
        .expect("fix");
    assert_eq!(&GUIDE[fix.range], "measure @a \"Proposition A\"");
    assert!(
        apply(GUIDE, &fix).ends_with("  measure @a \"Proposition A\"\n    recommend no-position\n")
    );
}

#[test]
fn an_unused_top_level_declaration_is_removed() {
    let fix = fixes(GUIDE)
        .into_iter()
        .find(|fix| fix.code == "unused-declaration")
        .expect("fix");
    assert_eq!(fix.title, "Remove `@gop`");
    let fixed = apply(GUIDE, &fix);
    assert!(!fixed.contains("@gop"));
    assert!(fixed.contains("party @dem \"Democratic\"\ncandidate @delgado"));
}

#[test]
fn diagnostics_without_a_repair_have_no_fix() {
    let fixes = fixes("oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-02-30\n");
    assert!(
        fixes
            .iter()
            .all(|fix| fix.code != "invalid-date" && fix.code != "empty-document")
    );
}
