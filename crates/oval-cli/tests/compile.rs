//! Runs the `oval compile` command against real files.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../oval-syntax/tests/fixtures/franklin.oval")
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
    assert_eq!(json["oval"], 1);
    assert_eq!(json["counts"]["contests"], 44);
    assert_eq!(json["counts"]["measures"], 28);
    assert_eq!(json["ballots"][0]["kind"], "ballot");
    assert_eq!(json["questions"].as_array().map(Vec::len), Some(72));
}

#[test]
fn pretty_output_is_indented() {
    let output = oval(&[
        "compile".as_ref(),
        "--pretty".as_ref(),
        fixture().as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("{\n  \"oval\": 1,\n"));
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
        stderr.contains("error[V2]: the document has no `election` field"),
        "{stderr}"
    );
    std::fs::remove_file(path).ok();
}
