#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jsonschema::Validator;
use oval_core::{CompileError, Report};

pub(crate) const FRANKLIN: &str = include_str!("../../../oval-syntax/tests/fixtures/franklin.oval");

pub(crate) const SHAPES: &str = include_str!("../fixtures/shapes.oval");

pub(crate) const COMPILE_FAILURES_AFTER_A_CLEAN_CHECK: &[&str] = &[
    "oval 99999999999\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n",
    "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @c \"C\" multiple 99999999999\n    campaign @a \"A\"\n    recommend\n      support @a\n",
];

static SCHEMA: LazyLock<Validator> = LazyLock::new(|| {
    let schema = serde_json::from_str(include_str!("../schema/oval.schema.json"))
        .expect("the vendored schema is JSON");
    jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .expect("the vendored schema is a valid JSON Schema")
});

pub(crate) fn check(source: &str) -> Report {
    let report = oval_core::check(source).expect("accepted");
    if report.has_errors() {
        return report;
    }
    let compiled = oval_core::compile(source);
    if COMPILE_FAILURES_AFTER_A_CLEAN_CHECK.contains(&source) {
        assert!(
            matches!(compiled, Err(CompileError::Invalid(_))),
            "a known compile failure now compiles; remove it from COMPILE_FAILURES_AFTER_A_CLEAN_CHECK:\n{source}"
        );
        return report;
    }
    match compiled {
        Ok(compiled) => assert_valid(&compiled.to_json().expect("serializes"), source),
        Err(error) => panic!("check found no errors but compile failed: {error:?}\n{source}"),
    }
    report
}

pub(crate) fn assert_valid(json: &str, source: &str) {
    let instance: serde_json::Value = serde_json::from_str(json).expect("compiled output is JSON");
    let evaluation = SCHEMA.evaluate(&instance);
    let errors: Vec<String> = evaluation
        .iter_errors()
        .map(|entry| {
            format!(
                "{} at {}: {}",
                entry.instance_location, entry.schema_location, entry.error
            )
        })
        .collect();
    assert!(
        evaluation.flag().valid,
        "compiled output does not validate against the schema:\n{}\n{source}",
        errors.join("\n")
    );
}

pub(crate) fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

pub(crate) fn assert_matches_golden(actual: &str, name: &str) {
    let path = fixture_path(name);
    if std::env::var_os("OVAL_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).expect("write golden");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        actual == expected,
        "{} differs from the compiled output; rerun with OVAL_UPDATE_GOLDEN=1 to regenerate it",
        path.display()
    );
}
