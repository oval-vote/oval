//! Checks the compiled output of the specification's example guide against the specification's
//! compiled example.
//!
//! TEMPORARY: this test reads the copies of the example guide and its output in
//! `crates/oval-syntax/tests/fixtures`, and exists to check the rewrite during development. Delete
//! it before the rewrite is merged.

use serde_json::Value;

const FRANKLIN: &str = include_str!("../../oval-syntax/tests/fixtures/franklin.oval");

const FRANKLIN_JSON: &str = include_str!("../../oval-syntax/tests/fixtures/franklin.oval.json");

fn differences(path: &str, actual: &Value, expected: &Value, found: &mut Vec<String>) {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            for (key, value) in expected {
                match actual.get(key) {
                    Some(other) => differences(&format!("{path}.{key}"), other, value, found),
                    None => found.push(format!("{path}.{key} is missing")),
                }
            }
            for key in actual.keys().filter(|key| !expected.contains_key(*key)) {
                found.push(format!("{path}.{key} is unexpected"));
            }
        }
        (Value::Array(actual), Value::Array(expected)) if actual.len() == expected.len() => {
            for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                differences(&format!("{path}[{index}]"), actual, expected, found);
            }
        }
        _ if actual != expected => {
            found.push(format!("{path} is {actual}, expected {expected}"));
        }
        _ => {}
    }
}

#[test]
fn the_example_guide_compiles_to_the_example_output() {
    let report = oval_core::check(FRANKLIN).expect("the example guide parses");
    let compiled = report.compile().expect("the example guide is valid");
    let actual: Value = serde_json::from_str(&compiled.to_json()).unwrap();
    let expected: Value = serde_json::from_str(FRANKLIN_JSON).unwrap();
    let mut found = Vec::new();
    differences("$", &actual, &expected, &mut found);
    assert!(
        found.is_empty(),
        "{} differences, the first are\n{}",
        found.len(),
        found
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
