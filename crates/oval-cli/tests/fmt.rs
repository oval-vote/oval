//! Runs the `oval fmt` command against real files.

mod common;

use common::{fixture, oval, temp_path, write_temp};

#[test]
fn check_passes_on_a_formatted_file() {
    let output = oval(&["fmt".as_ref(), "--check".as_ref(), fixture().as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
}

#[test]
fn check_names_files_that_would_change_and_leaves_them_alone() {
    let path = write_temp("dirty.oval", "guide   \"G\"\n");
    let output = oval(&["fmt".as_ref(), "--check".as_ref(), path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{} would be reformatted\n", path.display())
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "guide   \"G\"\n");
    std::fs::remove_file(path).ok();
}

#[test]
fn fmt_rewrites_in_place() {
    let path = write_temp(
        "rewrite.oval",
        "ballot @b \"B\"\n      measure   @m \"M\"\n            recommend yes",
    );
    let output = oval(&["fmt".as_ref(), path.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "ballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n"
    );
    std::fs::remove_file(path).ok();
}

#[test]
fn files_with_syntax_errors_are_reported_and_left_alone() {
    let path = write_temp("broken.oval", "oval 1\nguide yes\n");
    let output = oval(&["fmt".as_ref(), path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("error[unexpected-token]"));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "oval 1\nguide yes\n"
    );
    std::fs::remove_file(path).ok();
}

#[test]
fn a_valid_file_without_a_final_newline_stays_valid_after_formatting() {
    let path = write_temp(
        "unterminated.oval",
        "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n    subtitle \"S\" # c",
    );
    assert!(oval(&["check".as_ref(), path.as_os_str()]).status.success());
    let output = oval(&["fmt".as_ref(), path.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    let output = oval(&["check".as_ref(), path.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .ends_with("    subtitle \"S\" # c\n    recommend yes\n")
    );
    std::fs::remove_file(path).ok();
}

#[test]
fn a_missing_file_mid_batch_does_not_stop_the_others_being_formatted() {
    let first = write_temp("batch-first.oval", "guide   \"G\"\n");
    let missing = temp_path("batch-missing.oval");
    let last = write_temp("batch-last.oval", "guide   \"G\"\n");
    let output = oval(&[
        "fmt".as_ref(),
        first.as_os_str(),
        missing.as_os_str(),
        last.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "guide \"G\"\n");
    assert_eq!(std::fs::read_to_string(&last).unwrap(), "guide \"G\"\n");
    std::fs::remove_file(first).ok();
    std::fs::remove_file(last).ok();
}
