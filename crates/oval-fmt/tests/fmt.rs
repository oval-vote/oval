//! For every conformance case and example: formatting is idempotent, and it
//! preserves the data model and the set of diagnostic codes. Files with
//! syntax errors are refused.

use oval_fmt::{format, Options};
use std::path::{Path, PathBuf};

fn files(dir: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(dir);
    let mut v: Vec<PathBuf> = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "oval"))
        .collect();
    v.sort();
    v
}

fn codes(loaded: &oval_model::Loaded) -> Vec<&'static str> {
    let mut v: Vec<_> = loaded.diagnostics.iter().map(|d| d.code).collect();
    v.sort_unstable();
    v
}

#[test]
fn idempotent_and_model_preserving() {
    let opts = Options::default();
    let mut failures = Vec::new();
    for path in files("tests/cases").into_iter().chain(files("examples")) {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&path).unwrap();
        let before = oval_model::load(&text);
        let has_syntax_errors = !before.parse.diagnostics.is_empty();
        match format(&text, &opts) {
            Err(_) if has_syntax_errors => {}
            Err(d) => failures.push(format!("{name}: refused without syntax errors: {d:?}")),
            Ok(_) if has_syntax_errors => failures.push(format!("{name}: formatted despite syntax errors")),
            Ok(once) => {
                let twice =
                    format(&once, &opts).unwrap_or_else(|e| panic!("{name}: second pass failed: {e:?}"));
                if once != twice {
                    failures.push(format!("{name}: not idempotent\n--- once\n{once}\n--- twice\n{twice}"));
                }
                let after = oval_model::load(&once);
                let (a, b) = (
                    serde_json::to_value(&before.guide).unwrap(),
                    serde_json::to_value(&after.guide).unwrap(),
                );
                if a != b {
                    failures.push(format!("{name}: model changed\n--- before\n{a:#}\n--- after\n{b:#}"));
                }
                if codes(&before) != codes(&after) {
                    failures.push(format!(
                        "{name}: diagnostics changed: {:?} -> {:?}",
                        codes(&before),
                        codes(&after)
                    ));
                }
                if !once.ends_with('\n') || once.contains("\n\n\n") || once.lines().any(|l| l.ends_with(' '))
                {
                    failures.push(format!("{name}: whitespace not canonical"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn wraps_and_guards_reserved_starts() {
    let src = "oval 1\nname: T\ndate: 2026-11-03\nauthor: A\n\n# P @p\n\n## M @m\nmeasure\nvote: yes\n\nword word word word word word word word word word word word word word word word #hashtag\n";
    let out = format(src, &Options { width: 80 }).unwrap();
    assert!(out.contains("\n\\#hashtag\n"), "{out}");
    let again = oval_model::load(&out);
    assert!(again.diagnostics.is_empty(), "{:?}", again.diagnostics);
}
