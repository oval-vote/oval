//! The compiled fixtures against the specification's schema and their committed goldens.

mod support;

use support::{COMPILE_FAILURES_AFTER_A_CLEAN_CHECK, FRANKLIN, SHAPES};

fn compiles_cleanly_and_matches(source: &str, golden: &str) {
    assert_eq!(support::check(source).diagnostics(), []);
    let compiled = oval_core::compile(source).expect("compiles");
    support::assert_valid(&compiled.to_json().expect("serializes"), golden);
    let pretty = compiled.to_json_pretty().expect("serializes");
    support::assert_matches_golden(&format!("{pretty}\n"), golden);
}

#[test]
fn franklin_validates_against_the_schema_and_matches_its_golden() {
    compiles_cleanly_and_matches(FRANKLIN, "franklin.compiled.json");
}

#[test]
fn shapes_validates_against_the_schema_and_matches_its_golden() {
    compiles_cleanly_and_matches(SHAPES, "shapes.compiled.json");
}

#[test]
fn compact_and_pretty_output_are_the_same_value() {
    let compiled = oval_core::compile(SHAPES).expect("compiles");
    let compact: serde_json::Value =
        serde_json::from_str(&compiled.to_json().expect("serializes")).expect("JSON");
    let pretty: serde_json::Value =
        serde_json::from_str(&compiled.to_json_pretty().expect("serializes")).expect("JSON");
    assert_eq!(compact, pretty);
}

#[test]
fn known_compile_failures_after_a_clean_check_still_fail() {
    for source in COMPILE_FAILURES_AFTER_A_CLEAN_CHECK {
        assert!(!support::check(source).has_errors(), "{source}");
    }
}
