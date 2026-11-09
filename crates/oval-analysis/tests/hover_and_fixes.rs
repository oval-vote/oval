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
fn a_missing_guide_field_is_added_after_the_marker_with_its_value_as_a_placeholder() {
    let cases = [
        ("election", "election \"\"", "", Some("empty-string")),
        ("guide", "guide \"\"", "", Some("empty-string")),
        ("date", "date YYYY-MM-DD", "YYYY-MM-DD", None),
    ];
    let complete = ["guide \"G\"", "election \"E\"", "date 2026-11-03"];
    for (name, line, placeholder, warning) in cases {
        let rest: Vec<&str> = complete
            .into_iter()
            .filter(|field| !field.starts_with(name))
            .collect();
        let source = format!("oval 1\n{}\n", rest.join("\n"));
        let fix = fixes(&source)
            .into_iter()
            .find(|fix| fix.code == "missing-guide-field")
            .expect(name);
        assert_eq!(fix.title, format!("Add `{name}`"));
        let [edit] = fix.edits.as_slice() else {
            panic!("one edit for {name}");
        };
        assert_eq!(edit.text, format!("\n{line}"));
        let [range] = edit.placeholders.as_slice() else {
            panic!("one placeholder for {name}");
        };
        assert_eq!(&edit.text[*range], placeholder);
        let applied = apply(&source, &fix);
        assert!(
            applied.starts_with(&format!("oval 1\n{line}\n")),
            "{applied}"
        );
        let report = oval_core::check(&applied).expect("readable");
        let codes: Vec<&str> = report
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.identifier())
            .collect();
        assert!(!codes.contains(&"missing-guide-field"), "{codes:?}");
        if let Some(warning) = warning {
            assert!(
                codes.contains(&warning),
                "{name} left empty draws {warning}: {codes:?}"
            );
        }
    }
}

#[test]
fn ordinary_edits_have_no_placeholders() {
    assert!(
        fixes(GUIDE)
            .iter()
            .flat_map(|fix| &fix.edits)
            .all(|edit| edit.placeholders.is_empty())
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

#[test]
fn hovering_past_the_end_or_inside_a_character_does_not_panic() {
    let end = TextSize::try_from(GUIDE.len()).expect("fits");
    assert_eq!(hover(GUIDE, end + TextSize::from(1)), hover(GUIDE, end));
    assert_eq!(hover(GUIDE, TextSize::from(u32::MAX)), hover(GUIDE, end));
    let source = "oval 1\ncandidate @jose \"José\"\n";
    let accent = source.find('é').expect("accent");
    let inside = TextSize::try_from(accent + 1).expect("fits");
    let before = TextSize::try_from(accent).expect("fits");
    assert_eq!(hover(source, inside), hover(source, before));
}

fn with_terminator(source: &str, terminator: &str) -> String {
    source.replace('\n', terminator)
}

#[test]
fn inserted_lines_use_the_documents_own_line_terminator() {
    let source = "oval 1\nguide \"G\"\ndate 2026-11-03\nballot @b \"B\"\n  section \"S\"\n    measure @m \"M\"\n";
    let expected = [
        (
            "missing-guide-field",
            "oval 1\nelection \"\"\nguide \"G\"\ndate 2026-11-03\nballot @b \"B\"\n  section \"S\"\n    measure @m \"M\"\n",
        ),
        (
            "missing-recommendation",
            "oval 1\nguide \"G\"\ndate 2026-11-03\nballot @b \"B\"\n  section \"S\"\n    measure @m \"M\"\n      recommend no-position\n",
        ),
    ];
    for terminator in ["\n", "\r\n", "\r"] {
        let document = with_terminator(source, terminator);
        for (code, fixed) in expected {
            let fix = fixes(&document)
                .into_iter()
                .find(|fix| fix.code == code)
                .expect(code);
            let applied = apply(&document, &fix);
            assert_eq!(
                applied,
                with_terminator(fixed, terminator),
                "{code} with {terminator:?}"
            );
            let report = oval_core::check(&applied).expect("readable");
            assert!(
                report
                    .diagnostics()
                    .iter()
                    .all(|diagnostic| diagnostic.code.identifier() != code),
                "{code} remains after its fix with {terminator:?}"
            );
        }
    }
}

#[test]
fn a_candidate_whose_inline_party_is_named_elsewhere_is_not_removed() {
    let source = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\ncandidate @x \"X\"\n  party @rep \"Republican\"\ncandidate @y \"Y\"\n  party @rep\nballot @b \"B\"\n  contest @c \"C\" single\n    campaign @y\n    recommend\n      support @y\n";
    let report = oval_core::check(source).expect("readable");
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.identifier() == "unused-declaration"),
        "the candidate is unused"
    );
    assert!(
        fixes(source)
            .iter()
            .all(|fix| fix.code != "unused-declaration"),
        "removing @x would leave @rep undeclared"
    );
}
