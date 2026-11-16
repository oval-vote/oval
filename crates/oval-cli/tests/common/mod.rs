use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub(crate) const VALID: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n";

pub(crate) struct Run {
    pub(crate) status: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn run(test: &str, files: &[(&str, &[u8])], arguments: &[&str]) -> Run {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(arguments.first().copied().unwrap_or("none"))
        .join(test);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    for (name, contents) in files {
        fs::write(directory.join(name), contents).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_oval"))
        .args(arguments)
        .current_dir(&directory)
        .output()
        .unwrap();
    Run {
        status: output.status.code(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}
