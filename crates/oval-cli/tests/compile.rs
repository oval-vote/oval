//! Runs the `oval compile` command against real files.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../oval-syntax/tests/fixtures/franklin.oval")
}

fn golden() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../oval-core/tests/fixtures/franklin.compiled.json"),
    )
    .expect("read the compiled golden")
}

fn oval(args: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_oval"))
        .args(args)
        .output()
        .expect("run oval")
}

#[test]
fn the_fixture_compiles_to_json_on_stdout() {
    let output = oval(&["compile".as_ref(), fixture().as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    let expected: serde_json::Value = serde_json::from_str(&golden()).expect("golden is JSON");
    assert_eq!(json, expected);
    assert!(!output.stdout.starts_with(b"{\n"));
}

#[test]
fn pretty_output_matches_the_golden_byte_for_byte() {
    let output = oval(&[
        "compile".as_ref(),
        "--pretty".as_ref(),
        fixture().as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), golden());
}

#[test]
fn a_document_with_errors_prints_diagnostics_on_stderr_and_no_json() {
    let path = std::env::temp_dir().join(format!("oval-compile-{}.oval", std::process::id()));
    std::fs::write(&path, "oval 1\nguide \"G\"\n").expect("temp file");
    let output = oval(&["compile".as_ref(), path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error[missing-guide-field]: the document has no `election` field"),
        "{stderr}"
    );
    std::fs::remove_file(path).ok();
}

#[test]
fn json_diagnostics_go_to_stderr_when_compiling_fails() {
    let path = std::env::temp_dir().join(format!("oval-compile-json-{}.oval", std::process::id()));
    std::fs::write(&path, "oval 1\nguide \"G\"\n").expect("temp file");
    let output = oval(&["compile".as_ref(), "--json".as_ref(), path.as_os_str()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("stderr is a JSON array");
    let codes: Vec<&str> = json
        .as_array()
        .expect("array")
        .iter()
        .map(|entry| entry["code"].as_str().expect("code"))
        .collect();
    assert_eq!(
        codes,
        [
            "missing-guide-field",
            "missing-guide-field",
            "empty-document"
        ]
    );
    std::fs::remove_file(path).ok();
}
