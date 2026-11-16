//! Runs `oval compile` on files and checks what it prints and its exit status.

mod common;

use common::{VALID, run};

fn compiled(text: &str) -> oval_core::model::Compiled {
    oval_core::check(text).unwrap().compile().unwrap()
}

#[test]
fn a_valid_file_compiles_to_its_json_on_standard_output() {
    let run = run(
        "valid",
        &[("guide.oval", VALID.as_bytes())],
        &["compile", "guide.oval"],
    );
    assert_eq!(run.stdout, format!("{}\n", compiled(VALID).to_json()));
    assert_eq!(run.stderr, "");
    assert_eq!(run.status, Some(0));
}

#[test]
fn pretty_json_is_indented() {
    let run = run(
        "pretty",
        &[("guide.oval", VALID.as_bytes())],
        &["compile", "guide.oval", "--pretty"],
    );
    assert_eq!(
        run.stdout,
        format!("{}\n", compiled(VALID).to_json_pretty())
    );
    assert_eq!(run.status, Some(0));
}

#[test]
fn a_file_with_only_warnings_compiles_and_its_warnings_go_to_standard_error() {
    let text = VALID.replace("ballot @b", "candidate @c \"C\"\nballot @b");
    let run = run(
        "warnings",
        &[("guide.oval", text.as_bytes())],
        &["compile", "guide.oval"],
    );
    assert_eq!(run.stdout, format!("{}\n", compiled(&text).to_json()));
    assert_eq!(
        run.stderr,
        "guide.oval:5:1: warning[unused-declaration]: nothing refers to this declaration\n"
    );
    assert_eq!(run.status, Some(0));
}

#[test]
fn a_file_with_an_error_prints_its_diagnostics_and_no_json() {
    let text = VALID.replace("2026-11-03", "2026-02-30");
    let run = run(
        "invalid",
        &[("guide.oval", text.as_bytes())],
        &["compile", "guide.oval"],
    );
    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "guide.oval:4:6: error[invalid-date]: `2026-02-30` isn't a calendar date\n"
    );
    assert_eq!(run.status, Some(1));
}

#[test]
fn a_file_that_cannot_be_read_is_reported_on_standard_error() {
    let run = run("missing", &[], &["compile", "missing.oval"]);
    assert_eq!(run.stdout, "");
    assert!(
        run.stderr.starts_with("oval: missing.oval: "),
        "{}",
        run.stderr
    );
    assert_eq!(run.status, Some(2));
}

#[test]
fn compile_takes_exactly_one_file() {
    for arguments in [&["compile"][..], &["compile", "a.oval", "b.oval"]] {
        let run = run("usage", &[], arguments);
        assert_eq!(run.status, Some(2), "{arguments:?}");
        assert!(run.stderr.contains("Usage: oval compile"), "{}", run.stderr);
    }
}
