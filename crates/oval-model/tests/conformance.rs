//! Runs every file in `tests/cases` (repo root) through parse and lower.
//!
//! For each `<name>.oval`:
//! - the syntax tree must reproduce the source byte for byte;
//! - the diagnostics must match `<name>.diagnostics`, one `line code` per
//!   line (an absent or empty sidecar means no diagnostics at all);
//! - the lowered model is snapshotted as `<name>.snap` next to the case.
//!
//! `examples/*.oval` must parse with no errors (warnings are allowed).

use oval_syntax::{LineIndex, Severity};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn oval_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "oval"))
        .collect();
    v.sort();
    v
}

fn diagnostic_lines(text: &str, loaded: &oval_model::Loaded) -> Vec<String> {
    let index = LineIndex::new(text);
    let mut v: Vec<String> = loaded
        .diagnostics
        .iter()
        .map(|d| format!("{} {}", index.line_col(text, d.range.start()).0, d.code))
        .collect();
    v.sort();
    v
}

#[test]
fn cases() {
    let dir = repo_root().join("tests/cases");
    let mut failures = Vec::new();
    for path in oval_files(&dir) {
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let text = std::fs::read_to_string(&path).unwrap();
        let loaded = oval_model::load(&text);

        let round_trip = loaded.parse.syntax().text().to_string();
        if round_trip != text {
            failures.push(format!("{name}: syntax tree does not round-trip the source"));
        }

        let sidecar = path.with_extension("diagnostics");
        let mut expected: Vec<String> = std::fs::read_to_string(&sidecar)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.split_whitespace().take(2).collect::<Vec<_>>().join(" "))
            .collect();
        expected.sort();
        let actual = diagnostic_lines(&text, &loaded);
        if actual != expected {
            let index = LineIndex::new(&text);
            let detail: Vec<String> = loaded
                .diagnostics
                .iter()
                .map(|d| {
                    let (l, c) = index.line_col(&text, d.range.start());
                    format!("    {l}:{c} {} {}", d.code, d.message)
                })
                .collect();
            failures.push(format!("{name}: expected {expected:?}\n  actual:\n{}", detail.join("\n")));
        }

        insta::with_settings!({ snapshot_path => dir.clone(), prepend_module_to_snapshot => false, omit_expression => true }, {
            insta::assert_json_snapshot!(name, loaded.guide);
        });
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn examples_have_no_errors() {
    let dir = repo_root().join("examples");
    for path in oval_files(&dir) {
        let text = std::fs::read_to_string(&path).unwrap();
        let loaded = oval_model::load(&text);
        assert_eq!(loaded.parse.syntax().text().to_string(), text, "{}: round trip", path.display());
        let errors: Vec<_> = loaded.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "{}: {errors:#?}", path.display());
    }
}
