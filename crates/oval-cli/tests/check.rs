//! Runs `oval check` on files and checks what it prints and its exit status.

mod common;

use common::{VALID, run};

#[test]
fn a_valid_file_prints_nothing_and_succeeds() {
    let run = run(
        "valid",
        &[("guide.oval", VALID.as_bytes())],
        &["check", "guide.oval"],
    );
    assert_eq!(run.status, Some(0));
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "");
}

#[test]
fn each_diagnostic_is_a_line_with_its_position_severity_identifier_and_message() {
    let text = VALID
        .replace("2026-11-03", "2026-02-30")
        .replace("ballot @b", "candidate @c \"C\"\nballot @b");
    let run = run(
        "diagnostics",
        &[("guide.oval", text.as_bytes())],
        &["check", "guide.oval"],
    );
    assert_eq!(
        run.stdout,
        "guide.oval:4:6: error[invalid-date]: `2026-02-30` isn't a calendar date\nguide.oval:5:1: warning[unused-declaration]: nothing refers to this declaration\n"
    );
    assert_eq!(run.status, Some(1));
}

#[test]
fn warnings_alone_succeed() {
    let text = VALID.replace("ballot @b", "candidate @c \"C\"\nballot @b");
    let run = run(
        "warnings",
        &[("guide.oval", text.as_bytes())],
        &["check", "guide.oval"],
    );
    assert_eq!(
        run.stdout,
        "guide.oval:5:1: warning[unused-declaration]: nothing refers to this declaration\n"
    );
    assert_eq!(run.status, Some(0));
}

#[test]
fn a_column_counts_characters() {
    let text = VALID.replace("guide \"G\"", "guide \"Çé\" @x");
    let run = run(
        "columns",
        &[("guide.oval", text.as_bytes())],
        &["check", "guide.oval"],
    );
    assert!(
        run.stdout.starts_with("guide.oval:2:12: error["),
        "{}",
        run.stdout
    );
}

#[test]
fn every_file_is_checked_and_the_worst_outcome_is_the_status() {
    let invalid = VALID.replace("2026-11-03", "2026-02-30");
    let run = run(
        "several",
        &[
            ("valid.oval", VALID.as_bytes()),
            ("invalid.oval", invalid.as_bytes()),
        ],
        &["check", "valid.oval", "invalid.oval", "valid.oval"],
    );
    assert_eq!(
        run.stdout,
        "invalid.oval:4:6: error[invalid-date]: `2026-02-30` isn't a calendar date\n"
    );
    assert_eq!(run.status, Some(1));
}

#[test]
fn a_file_that_cannot_be_checked_is_reported_on_standard_error_and_the_rest_are_still_checked() {
    let invalid = VALID.replace("2026-11-03", "2026-02-30");
    let run = run(
        "unchecked",
        &[
            ("nul.oval", b"oval 1\0\n"),
            ("bytes.oval", b"oval 1\n\xff\n"),
            ("invalid.oval", invalid.as_bytes()),
        ],
        &[
            "check",
            "missing.oval",
            "nul.oval",
            "bytes.oval",
            "invalid.oval",
        ],
    );
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert_eq!(lines.len(), 3, "{}", run.stderr);
    assert!(lines[0].starts_with("oval: missing.oval: "), "{}", lines[0]);
    assert_eq!(
        lines[1],
        "oval: nul.oval: the document contains U+0000 at byte 6"
    );
    assert_eq!(
        lines[2],
        "oval: bytes.oval: the document is not valid UTF-8 at byte 7"
    );
    assert_eq!(
        run.stdout,
        "invalid.oval:4:6: error[invalid-date]: `2026-02-30` isn't a calendar date\n"
    );
    assert_eq!(run.status, Some(2));
}

#[test]
fn a_command_without_files_is_a_usage_error() {
    let run = run("usage", &[], &["check"]);
    assert_eq!(run.status, Some(2));
    assert!(run.stderr.contains("Usage: oval check"), "{}", run.stderr);
}
