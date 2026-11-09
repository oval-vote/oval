//! Runs the `oval check` command against real files.

mod common;

use common::{fixture, oval, temp_path, write_temp, write_temp_bytes};

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
fn strict_fails_on_warnings_alone() {
    let path = write_temp(
        "strict.oval",
        "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nparty @dem \"D\"\ncandidate @c \"C\"\n  party @dem\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n",
    );
    let output = oval(&["check".as_ref(), "--strict".as_ref(), path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("warning[unused-declaration]"));
    let clean = oval(&["check".as_ref(), "--strict".as_ref(), fixture().as_os_str()]);
    assert!(clean.status.success(), "{clean:?}");
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
            "range": { "start": 35, "end": 45 },
            "url": "https://spec.oval.vote/draft/#sec-Invalid-Date"
        }])
    );
    std::fs::remove_file(path).ok();

    let output = oval(&["check".as_ref(), "--json".as_ref(), fixture().as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "[]\n");
}

const INVALID_DATE: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-02-30\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n";

#[test]
fn a_missing_file_mid_batch_is_reported_and_the_rest_are_still_checked() {
    let first = write_temp("batch-first.oval", INVALID_DATE);
    let missing = temp_path("batch-missing.oval");
    let last = write_temp("batch-last.oval", INVALID_DATE);
    let files = [first.as_os_str(), missing.as_os_str(), last.as_os_str()];

    let output = oval(&[&["check".as_ref(), "--json".as_ref()], &files[..]].concat());
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is a JSON array");
    let paths: Vec<&str> = json
        .as_array()
        .expect("array")
        .iter()
        .map(|entry| entry["path"].as_str().expect("path"))
        .collect();
    assert_eq!(
        paths,
        [first.display().to_string(), last.display().to_string()]
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .collect::<Vec<_>>()
            .len(),
        1
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .starts_with(&format!("oval: cannot read {}: ", missing.display()))
    );

    let output = oval(&[&["check".as_ref()], &files[..]].concat());
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("{}:4:6: error[invalid-date]", first.display())));
    assert!(stdout.contains(&format!("{}:4:6: error[invalid-date]", last.display())));

    std::fs::remove_file(first).ok();
    std::fs::remove_file(last).ok();
}

#[test]
fn a_file_with_a_nul_byte_is_reported_and_the_rest_are_still_checked() {
    let rejected = write_temp("nul.oval", "oval 1\n\0\n");
    let last = write_temp("after-nul.oval", INVALID_DATE);
    let output = oval(&["check".as_ref(), rejected.as_os_str(), last.as_os_str()]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .starts_with(&format!("oval: {}: ", rejected.display()))
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!("{}:4:6: error[invalid-date]", last.display()))
    );
    std::fs::remove_file(rejected).ok();
    std::fs::remove_file(last).ok();
}

#[test]
fn invalid_utf8_is_reported_at_its_first_bad_byte() {
    let path = write_temp_bytes("latin1.oval", b"oval 1\nguide \"G\xe9\"\n");
    let output = oval(&["check".as_ref(), path.as_os_str()]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("oval: {}:2:9: not valid UTF-8\n", path.display())
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    std::fs::remove_file(path).ok();
}
