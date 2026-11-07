//! Runs the `oval check` command against real files.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../oval-syntax/tests/fixtures/franklin.oval")
}

fn write_temp(name: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("oval-cli-{}-{name}", std::process::id()));
    std::fs::write(&path, contents).expect("temp file");
    path
}

fn oval(args: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_oval"))
        .args(args)
        .output()
        .expect("run oval")
}

#[test]
fn a_clean_file_prints_nothing_and_succeeds() {
    let output = oval(&["check".as_ref(), fixture().as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn diagnostics_are_printed_with_positions_and_codes() {
    let path = write_temp(
        "broken.oval",
        "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-02-30\nparty @dem \"D\"\ncandidate @c \"C\"\n  party @dem\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n",
    );
    let output = oval(&["check".as_ref(), path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let shown = path.display();
    assert_eq!(
        stdout,
        format!(
            "{shown}:4:6: error[invalid-date]: 2026-02-30 is not a calendar date\n\
             {shown}:6:1: warning[unused-declaration]: `@c` is declared but is not in any campaign\n"
        )
    );
    std::fs::remove_file(path).ok();
}

#[test]
fn warnings_alone_do_not_fail() {
    let path = write_temp(
        "warn.oval",
        "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"D\"\ncandidate @c \"C\"\n  party @dem\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n",
    );
    let output = oval(&["check".as_ref(), path.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("warning[unused-declaration]"));
    std::fs::remove_file(path).ok();
}

#[test]
fn an_unreadable_file_is_exit_code_two() {
    let output = oval(&["check".as_ref(), "/definitely/not/here.oval".as_ref()]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("oval: cannot read "));
}

#[test]
fn check_requires_at_least_one_file() {
    let output = oval(&["check".as_ref()]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn json_output_carries_positions_ranges_and_identifiers() {
    let path = write_temp(
        "json.oval",
        "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-02-30\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n",
    );
    let output = oval(&["check".as_ref(), "--json".as_ref(), path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is a JSON array");
    assert_eq!(
        json,
        serde_json::json!([{
            "path": path.display().to_string(),
            "code": "invalid-date",
            "severity": "error",
            "message": "2026-02-30 is not a calendar date",
            "start": { "line": 4, "column": 6 },
            "end": { "line": 4, "column": 16 },
            "range": { "start": 35, "end": 45 }
        }])
    );
    std::fs::remove_file(path).ok();

    let output = oval(&["check".as_ref(), "--json".as_ref(), fixture().as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "[]\n");
}
