//! Runs the `oval fmt` command against real files.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../oval-syntax/tests/fixtures/franklin.oval")
}

fn write_temp(name: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("oval-fmt-{}-{name}", std::process::id()));
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
