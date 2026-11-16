//! Runs `oval fmt` on files and checks what it changes, what it prints, and its exit status.

mod common;

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

use common::{VALID, run};

const MESSY: &str = "oval   1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n    measure @m \"M\"\n        recommend yes\n";

fn directory(test: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("fmt")
        .join(test)
}

fn contents(test: &str, file: &str) -> String {
    fs::read_to_string(directory(test).join(file)).unwrap()
}

fn modified(test: &str, file: &str) -> SystemTime {
    fs::metadata(directory(test).join(file))
        .unwrap()
        .modified()
        .unwrap()
}

fn set_modified(test: &str, file: &str, time: SystemTime) {
    File::options()
        .write(true)
        .open(directory(test).join(file))
        .unwrap()
        .set_modified(time)
        .unwrap();
}

fn while_read_only<T>(test: &str, file: &str, body: impl FnOnce() -> T) -> T {
    let path = directory(test).join(file);
    let original = fs::metadata(&path).unwrap().permissions();
    let mut read_only = original.clone();
    read_only.set_readonly(true);
    fs::set_permissions(&path, read_only).unwrap();
    let result = body();
    fs::set_permissions(&path, original).unwrap();
    result
}

fn rerun(test: &str, arguments: &[&str]) -> Option<i32> {
    Command::new(env!("CARGO_BIN_EXE_oval"))
        .args(arguments)
        .current_dir(directory(test))
        .status()
        .unwrap()
        .code()
}

#[test]
fn each_file_is_rewritten_in_the_canonical_style() {
    let run = run(
        "rewritten",
        &[("a.oval", MESSY.as_bytes()), ("b.oval", MESSY.as_bytes())],
        &["fmt", "a.oval", "b.oval"],
    );
    assert_eq!((run.stdout.as_str(), run.stderr.as_str()), ("", ""));
    assert_eq!(run.status, Some(0));
    assert_eq!(contents("rewritten", "a.oval"), VALID);
    assert_eq!(contents("rewritten", "b.oval"), VALID);
}

#[test]
fn a_formatted_file_is_not_written() {
    let run = run(
        "formatted",
        &[("guide.oval", VALID.as_bytes())],
        &["fmt", "guide.oval"],
    );
    assert_eq!(run.status, Some(0));
    let long_ago = SystemTime::UNIX_EPOCH + Duration::from_hours(24);
    set_modified("formatted", "guide.oval", long_ago);
    assert_eq!(rerun("formatted", &["fmt", "guide.oval"]), Some(0));
    assert_eq!(modified("formatted", "guide.oval"), long_ago);
}

#[test]
fn check_lists_the_files_that_would_change_and_changes_nothing() {
    let run = run(
        "check",
        &[
            ("messy.oval", MESSY.as_bytes()),
            ("tidy.oval", VALID.as_bytes()),
        ],
        &["fmt", "--check", "messy.oval", "tidy.oval"],
    );
    assert_eq!(run.stdout, "messy.oval\n");
    assert_eq!(run.status, Some(1));
    assert_eq!(contents("check", "messy.oval"), MESSY);
}

#[test]
fn check_on_formatted_files_succeeds_quietly() {
    let run = run(
        "tidy",
        &[("guide.oval", VALID.as_bytes())],
        &["fmt", "--check", "guide.oval"],
    );
    assert_eq!((run.stdout.as_str(), run.stderr.as_str()), ("", ""));
    assert_eq!(run.status, Some(0));
}

#[test]
fn a_file_with_syntax_errors_is_left_as_it_is_and_its_errors_are_reported() {
    let text = "guide \"G\n";
    let run = run(
        "malformed",
        &[
            ("guide.oval", text.as_bytes()),
            ("messy.oval", MESSY.as_bytes()),
        ],
        &["fmt", "guide.oval", "messy.oval"],
    );
    assert_eq!(
        run.stderr,
        "guide.oval:1:7: error[unterminated-string]: the string has no closing quote\n"
    );
    assert_eq!(run.status, Some(1));
    assert_eq!(contents("malformed", "guide.oval"), text);
    assert_eq!(contents("malformed", "messy.oval"), VALID);
}

#[test]
fn a_file_that_cannot_be_read_or_written_is_reported_and_the_rest_are_still_formatted() {
    let run = run(
        "problems",
        &[
            ("nul.oval", b"oval 1\0\n"),
            ("locked.oval", MESSY.as_bytes()),
            ("messy.oval", MESSY.as_bytes()),
        ],
        &["fmt", "missing.oval", "nul.oval", "messy.oval"],
    );
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{}", run.stderr);
    assert!(lines[0].starts_with("oval: missing.oval: "), "{}", lines[0]);
    assert_eq!(
        lines[1],
        "oval: nul.oval: the document contains U+0000 at byte 6"
    );
    assert_eq!(run.status, Some(2));
    assert_eq!(contents("problems", "messy.oval"), VALID);
    let status = while_read_only("problems", "locked.oval", || {
        rerun("problems", &["fmt", "locked.oval"])
    });
    assert_eq!(status, Some(2));
    assert_eq!(contents("problems", "locked.oval"), MESSY);
}
