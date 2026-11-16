//! Runs each command with `--json` and checks the array of diagnostics it prints.

mod common;

use common::{VALID, run};
use serde_json::{Value, json};

fn invalid() -> String {
    VALID.replace("2026-11-03", "2026-02-30")
}

fn invalid_date(path: &str) -> Value {
    json!({
        "path": path,
        "code": "invalid-date",
        "severity": "error",
        "message": "`2026-02-30` isn't a calendar date",
        "start": { "line": 4, "column": 6 },
        "end": { "line": 4, "column": 16 },
        "range": { "start": 35, "end": 45 },
    })
}

fn parsed(output: &str) -> Value {
    assert!(output.ends_with('\n'), "{output:?}");
    assert_eq!(output.lines().count(), 1, "{output:?}");
    serde_json::from_str(output).unwrap()
}

#[test]
fn check_prints_one_array_for_every_file() {
    let warned = VALID.replace("ballot @b", "candidate @c \"C\"\nballot @b");
    let run = run(
        "check",
        &[
            ("invalid.oval", invalid().as_bytes()),
            ("warned.oval", warned.as_bytes()),
        ],
        &["check", "--json", "invalid.oval", "warned.oval"],
    );
    assert_eq!(
        parsed(&run.stdout),
        json!([
            invalid_date("invalid.oval"),
            {
                "path": "warned.oval",
                "code": "unused-declaration",
                "severity": "warning",
                "message": "nothing refers to this declaration",
                "start": { "line": 5, "column": 1 },
                "end": { "line": 5, "column": 17 },
                "range": { "start": 46, "end": 62 },
            },
        ])
    );
    assert_eq!(run.status, Some(1));
}

#[test]
fn check_prints_an_empty_array_when_there_is_nothing_to_report() {
    let run = run(
        "check-empty",
        &[("valid.oval", VALID.as_bytes())],
        &["check", "--json", "valid.oval", "missing.oval"],
    );
    assert_eq!(run.stdout, "[]\n");
    assert!(
        run.stderr.starts_with("oval: missing.oval: "),
        "{}",
        run.stderr
    );
    assert_eq!(run.status, Some(2));
}

#[test]
fn compile_prints_its_array_on_standard_error_only_when_there_is_something_to_report() {
    let invalid = run(
        "compile",
        &[("invalid.oval", invalid().as_bytes())],
        &["compile", "--json", "invalid.oval"],
    );
    assert_eq!(invalid.stdout, "");
    assert_eq!(
        parsed(&invalid.stderr),
        json!([invalid_date("invalid.oval")])
    );
    assert_eq!(invalid.status, Some(1));
    let valid = run(
        "compile-valid",
        &[("valid.oval", VALID.as_bytes())],
        &["compile", "--json", "valid.oval"],
    );
    assert_eq!(valid.stderr, "");
    assert_eq!(valid.status, Some(0));
}

#[test]
fn fmt_prints_its_array_on_standard_error_only_when_there_is_something_to_report() {
    let malformed = run(
        "fmt",
        &[("guide.oval", b"guide \"G\n")],
        &["fmt", "--json", "guide.oval"],
    );
    assert_eq!(
        parsed(&malformed.stderr),
        json!([{
            "path": "guide.oval",
            "code": "unterminated-string",
            "severity": "error",
            "message": "the string has no closing quote",
            "start": { "line": 1, "column": 7 },
            "end": { "line": 1, "column": 9 },
            "range": { "start": 6, "end": 8 },
        }])
    );
    assert_eq!(malformed.status, Some(1));
    let valid = run(
        "fmt-valid",
        &[("valid.oval", VALID.as_bytes())],
        &["fmt", "--json", "valid.oval"],
    );
    assert_eq!(valid.stderr, "");
    assert_eq!(valid.status, Some(0));
}
