//! Runs the `oval compile` command against real files.

mod common;

use common::{fixture, oval};

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

#[cfg(unix)]
#[test]
fn closing_stdout_early_ends_the_run_quietly() {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_oval"))
        .args([
            "compile".as_ref(),
            "--pretty".as_ref(),
            fixture().as_os_str(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run oval");
    drop(child.stdout.take());
    let output = child.wait_with_output().expect("wait for oval");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[cfg(target_os = "linux")]
#[test]
fn a_full_disk_is_exit_code_two() {
    let full = std::fs::File::create("/dev/full").expect("open /dev/full");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_oval"))
        .args(["check".as_ref(), "--json".as_ref(), fixture().as_os_str()])
        .stdout(full)
        .output()
        .expect("run oval");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}
