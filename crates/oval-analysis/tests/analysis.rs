//! Tests of the symbols and the outline over small documents and the shared fixture.

use oval_analysis::{OutlineItem, OutlineKind, Symbol, SymbolKind, Symbols, outline};
use oval_syntax::TextSize;
use oval_syntax::ast::{AstNode, Document};
use oval_syntax::parser::parse;

const FIXTURE: &str = include_str!("../../oval-syntax/tests/fixtures/franklin.oval");

fn document(source: &str) -> Document {
    Document::cast(parse(source).expect("accepted").syntax()).expect("a document")
}

fn offset(source: &str, needle: &str, occurrence: usize) -> TextSize {
    let mut from = 0;
    for _ in 0..occurrence {
        from = source[from..].find(needle).expect("occurrence") + from + needle.len();
    }
    let at = source[from..].find(needle).expect("occurrence") + from;
    TextSize::try_from(at).expect("fits")
}

fn count(items: &[OutlineItem], kind: OutlineKind) -> usize {
    items
        .iter()
        .map(|item| usize::from(item.kind == kind) + count(&item.children, kind))
        .sum()
}

const SMALL: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\n\
party @dem \"Democratic\"\n\
candidate @delgado \"Rosa Delgado\"\n  party @dem\n\
ballot @city \"City\"\n  section \"Offices\"\n    contest @attorney \"City Attorney\" single\n      campaign @delgado\n      campaign @webb \"Marcus Webb\"\n        party @dem\n      recommend\n        support @delgado\n        oppose @webb\n  measure @a \"Proposition A\"\n    recommend yes\n";

#[test]
fn declarations_and_references_are_in_document_order_with_their_kinds() {
    let symbols = Symbols::new(&document(SMALL));
    let declared: Vec<(&str, SymbolKind)> = symbols
        .declarations
        .iter()
        .map(|d| (d.name.as_str(), d.kind))
        .collect();
    assert_eq!(
        declared,
        [
            ("dem", SymbolKind::Party),
            ("delgado", SymbolKind::Candidate),
            ("city", SymbolKind::Ballot),
            ("attorney", SymbolKind::Contest),
            ("webb", SymbolKind::Candidate),
            ("a", SymbolKind::Measure),
        ]
    );
    let referenced: Vec<(&str, Option<usize>)> = symbols
        .references
        .iter()
        .map(|r| (r.name.as_str(), r.declaration))
        .collect();
    assert_eq!(
        referenced,
        [
            ("dem", Some(0)),
            ("delgado", Some(1)),
            ("dem", Some(0)),
            ("delgado", Some(1)),
            ("webb", Some(4)),
        ]
    );
}

#[test]
fn ranges_cover_the_identifier_and_the_name_alone() {
    let symbols = Symbols::new(&document(SMALL));
    let delgado = &symbols.declarations[1];
    assert_eq!(&SMALL[delgado.range], "@delgado");
    assert_eq!(&SMALL[delgado.name_range], "delgado");
    let last = symbols.references.last().unwrap();
    assert_eq!(&SMALL[last.range], "@webb");
    assert_eq!(&SMALL[last.name_range], "webb");
}

#[test]
fn the_cursor_finds_an_identifier_at_either_end() {
    let symbols = Symbols::new(&document(SMALL));
    let start = offset(SMALL, "@delgado", 1);
    assert_eq!(symbols.at(start), Some(Symbol::Reference(1)));
    assert_eq!(
        symbols.at(start + TextSize::from(8)),
        Some(Symbol::Reference(1))
    );
    assert_eq!(symbols.at(start + TextSize::from(9)), None);
    assert_eq!(symbols.at(offset(SMALL, "guide", 0)), None);
}

#[test]
fn definition_goes_from_any_occurrence_to_the_declaration() {
    let symbols = Symbols::new(&document(SMALL));
    let declaration = offset(SMALL, "@delgado", 0);
    for occurrence in 0..3 {
        let definition = symbols
            .definition(offset(SMALL, "@delgado", occurrence))
            .unwrap();
        assert_eq!(definition.range.start(), declaration);
        assert_eq!(definition.kind, SymbolKind::Candidate);
    }
}

#[test]
fn occurrences_list_the_declaration_and_every_reference() {
    let symbols = Symbols::new(&document(SMALL));
    let all = symbols.occurrences(offset(SMALL, "@delgado", 2));
    assert_eq!(all.len(), 3);
    assert!(all.iter().all(|range| &SMALL[*range] == "@delgado"));
    assert!(all.windows(2).all(|pair| pair[0].start() < pair[1].start()));
    let names = symbols.occurrence_names(offset(SMALL, "@delgado", 2));
    assert!(names.iter().all(|range| &SMALL[*range] == "delgado"));
    assert_eq!(symbols.occurrences(offset(SMALL, "guide", 0)), []);
}

#[test]
fn an_undeclared_reference_has_no_definition_but_is_still_an_occurrence() {
    let source = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\n\
ballot @b \"B\"\n  contest @c \"C\" single\n    campaign @nobody\n    recommend\n      support @nobody\n";
    let symbols = Symbols::new(&document(source));
    let at = offset(source, "@nobody", 1);
    assert_eq!(
        symbols
            .references
            .iter()
            .filter(|r| r.name == "nobody")
            .count(),
        2
    );
    assert!(symbols.definition(at).is_none());
    assert_eq!(symbols.occurrences(at).len(), 2);
}

#[test]
fn the_outline_nests_ballots_sections_and_questions() {
    let items = outline(&document(SMALL));
    assert_eq!(items.len(), 1);
    let city = &items[0];
    assert_eq!(
        (city.kind, city.title.as_deref(), city.name.as_deref()),
        (OutlineKind::Ballot, Some("City"), Some("city"))
    );
    assert_eq!(&SMALL[city.selection], "ballot @city \"City\"");
    assert_eq!(city.children.len(), 2);
    let offices = &city.children[0];
    assert_eq!(
        (
            offices.kind,
            offices.title.as_deref(),
            offices.name.as_deref()
        ),
        (OutlineKind::Section, Some("Offices"), None)
    );
    assert_eq!(offices.children[0].title.as_deref(), Some("City Attorney"));
    assert_eq!(offices.children[0].kind, OutlineKind::Contest);
    let measure = &city.children[1];
    assert_eq!(
        (measure.kind, measure.name.as_deref()),
        (OutlineKind::Measure, Some("a"))
    );
    assert!(city.range.contains_range(measure.range));
}

#[test]
fn the_fixture_resolves_every_reference_and_outlines_every_question() {
    let document = document(FIXTURE);
    let symbols = Symbols::new(&document);
    assert!(symbols.references.iter().all(|r| r.declaration.is_some()));
    let compiled = oval_core::compile(FIXTURE).expect("the fixture compiles");
    let items = outline(&document);
    assert_eq!(count(&items, OutlineKind::Ballot), compiled.counts.ballots);
    assert_eq!(
        count(&items, OutlineKind::Section),
        compiled.counts.sections
    );
    assert_eq!(
        count(&items, OutlineKind::Contest),
        compiled.counts.contests
    );
    assert_eq!(
        count(&items, OutlineKind::Measure),
        compiled.counts.measures
    );
}
