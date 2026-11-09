#![allow(dead_code)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../oval-syntax/tests/fixtures/franklin.oval")
}

pub(crate) fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("oval-cli-{}-{name}", std::process::id()))
}

pub(crate) fn write_temp(name: &str, contents: &str) -> PathBuf {
    write_temp_bytes(name, contents.as_bytes())
}

pub(crate) fn write_temp_bytes(name: &str, contents: &[u8]) -> PathBuf {
    let path = temp_path(name);
    std::fs::write(&path, contents).expect("temp file");
    path
}

pub(crate) fn oval(args: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oval"))
        .args(args)
        .output()
        .expect("run oval")
}
