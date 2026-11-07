//! Tests of completions at positions in small documents.

use oval_analysis::{Completion, CompletionKind, completions};
use oval_syntax::TextSize;

const GUIDE: &str = r#"oval 1
guide "G"
election "E"
date 2026-11-03
party @dem "Democratic"
party @gop "Republican"
candidate @delgado "Rosa Delgado"
  party @dem
ballot @city "City"
  contest @attorney "City Attorney" single
    campaign @delgado
    campaign @webb "Marcus Webb"
    recommend
      support @delgado
  measure @a "Proposition A"
    note:
      Some *text*.
    recommend yes
"#;

fn at(source: &str, needle: &str, occurrence: usize) -> TextSize {
    let mut from = 0;
    for _ in 0..occurrence {
        from = source[from..].find(needle).expect("occurrence") + from + needle.len();
    }
    TextSize::try_from(source[from..].find(needle).expect("occurrence") + from + needle.len())
        .expect("fits")
}

fn labels(found: &[Completion]) -> Vec<&str> {
    found.iter().map(|c| c.label.as_str()).collect()
}

#[test]
fn a_new_line_in_a_block_offers_the_entries_not_yet_written() {
    let source = GUIDE.replace("    campaign @delgado\n", "    campaign @delgado\n    \n");
    let found = completions(&source, at(&source, "    campaign @delgado\n    ", 0));
    assert_eq!(
        labels(&found),
        ["subtitle", "tabulation", "note", "campaign", "recommend"]
    );
    assert!(found.iter().all(|c| c.kind == CompletionKind::Keyword));
}

#[test]
fn the_top_level_offers_what_the_document_still_lacks() {
    let source = "oval 1\nguide \"G\"\n";
    let found = completions(source, TextSize::try_from(source.len()).unwrap());
    assert_eq!(
        labels(&found),
        ["election", "date", "note", "party", "candidate", "ballot"]
    );
}

#[test]
fn a_value_position_offers_its_literals() {
    let found = completions(GUIDE, at(GUIDE, "recommend yes", 0) - TextSize::from(3));
    assert_eq!(labels(&found), ["yes", "no", "no-position"]);
    let source = GUIDE.replace("\"City Attorney\" single", "\"City Attorney\" ");
    let found = completions(&source, at(&source, "\"City Attorney\" ", 0));
    assert_eq!(
        labels(&found),
        ["single", "multiple", "ranked", "retention"]
    );
}

#[test]
fn a_partial_word_is_the_range_the_completion_replaces() {
    let source = GUIDE.replace("\"City Attorney\" single", "\"City Attorney\" sin");
    let offset = at(&source, "\"City Attorney\" sin", 0);
    let found = completions(&source, offset);
    assert_eq!(
        labels(&found),
        ["single", "multiple", "ranked", "retention"]
    );
    assert_eq!(&source[found[0].replace], "sin");
}

#[test]
fn a_reference_position_offers_identifiers_of_the_right_kind_with_their_names() {
    let found = completions(GUIDE, at(GUIDE, "support ", 0));
    assert_eq!(labels(&found), ["@delgado", "@webb"]);
    assert_eq!(found[0].kind, CompletionKind::Candidate);
    assert_eq!(found[0].detail.as_deref(), Some("Rosa Delgado"));
    assert_eq!(found[1].detail.as_deref(), Some("Marcus Webb"));
    let found = completions(GUIDE, at(GUIDE, "  party ", 0));
    assert_eq!(labels(&found), ["@dem", "@gop"]);
    assert_eq!(found[0].kind, CompletionKind::Party);
}

#[test]
fn a_partial_identifier_keeps_its_at_sign_in_the_replaced_range() {
    let source = GUIDE.replace("support @delgado", "support @de");
    let found = completions(&source, at(&source, "support @de", 0));
    assert_eq!(labels(&found), ["@delgado", "@webb"]);
    assert_eq!(&source[found[0].replace], "@de");
}

#[test]
fn nothing_is_offered_inside_strings_bodies_and_comments_or_for_a_new_name() {
    assert_eq!(completions(GUIDE, at(GUIDE, "\"City Att", 0)), []);
    assert_eq!(completions(GUIDE, at(GUIDE, "Some *text*", 0)), []);
    let commented = GUIDE.replace("ballot @city", "# a comment\nballot @city");
    assert_eq!(completions(&commented, at(&commented, "# a comm", 0)), []);
    let source = GUIDE.replace("measure @a", "measure @");
    assert_eq!(completions(&source, at(&source, "measure @", 0)), []);
}
