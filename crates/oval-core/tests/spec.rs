//! Checks validation against the examples of the specification's validation errors and warnings.
//!
//! TEMPORARY: these tests read the copies of the specification and its example guide in
//! `crates/oval-syntax/tests/fixtures`, and exist to check the rewrite during development. Delete
//! them before the rewrite is merged.

use std::fmt::Write;

use oval_core::{DiagnosticCode, Report, check};
use oval_syntax::SyntaxErrorKind;

const SPEC: &str = include_str!("../../oval-syntax/tests/fixtures/SPEC.md");

const FRANKLIN: &str = include_str!("../../oval-syntax/tests/fixtures/franklin.oval");

const GUIDE: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\n";

const ROOT_BALLOT: &str = "ballot @zz \"Z\"\n  measure @zm \"Z\"\n    recommend yes\n";

const DECLARED_ELSEWHERE: &str =
    "candidate @nreyes \"Nadia Reyes\"\n  party @dem\ncandidate @cfox \"Carl Fox\"\n";

struct Example {
    counter: bool,
    content: String,
    line: usize,
}

fn examples(spec: &str) -> Vec<Example> {
    let mut examples = Vec::new();
    let mut lines = spec.lines().enumerate();
    while let Some((index, line)) = lines.next() {
        let Some(info) = line.strip_prefix("```") else {
            continue;
        };
        let mut content = String::new();
        for (_, line) in lines.by_ref() {
            if line == "```" {
                break;
            }
            content.push_str(line);
            content.push('\n');
        }
        if info == "oval example" || info == "oval counter-example" {
            examples.push(Example {
                counter: info == "oval counter-example",
                content,
                line: index + 1,
            });
        }
    }
    examples
}

fn entry(spec: &str, line: usize) -> Option<(String, String)> {
    let lines: Vec<&str> = spec.lines().take(line).collect();
    let section = lines
        .iter()
        .rev()
        .find_map(|text| text.strip_prefix("### "))?;
    if !section.starts_with("3.3.") && !section.starts_with("3.4.") {
        return None;
    }
    let heading = lines.iter().rposition(|text| text.starts_with("#### "))?;
    let text = lines[heading..].join(" ");
    let (_, after) = text.split_once("reported as `")?;
    let (identifier, _) = after.split_once('`')?;
    Some((lines[heading][5..].to_owned(), identifier.to_owned()))
}

fn declarations(guide: &str) -> String {
    let mut text = String::new();
    let mut inside = false;
    for line in guide.lines() {
        if line.starts_with("party ") || line.starts_with("candidate ") {
            inside = true;
        } else if !line.starts_with(' ') {
            inside = false;
        }
        if inside {
            text.push_str(line);
            text.push('\n');
        }
    }
    text.push_str(DECLARED_ELSEWHERE);
    text
}

fn used_elsewhere(example: &str) -> String {
    let mut text = String::from("ballot @zu \"Z\"\n");
    for (index, line) in example.lines().enumerate() {
        let campaign = if let Some(rest) = line.strip_prefix("party ") {
            format!(
                "campaign party {}",
                rest.split_whitespace().next().unwrap_or_default()
            )
        } else if let Some(rest) = line.strip_prefix("candidate ") {
            format!(
                "campaign {}",
                rest.split_whitespace().next().unwrap_or_default()
            )
        } else {
            continue;
        };
        write!(
            text,
            "  contest @zu{index} \"Z\" single\n    {campaign}\n    recommend no-position\n"
        )
        .expect("writing to a string succeeds");
    }
    text
}

fn indented(example: &str, indent: usize) -> String {
    let mut text = String::new();
    for line in example.lines() {
        if !line.is_empty() {
            text.push_str(&" ".repeat(indent));
        }
        text.push_str(line);
        text.push('\n');
    }
    text
}

fn contexts(example: &str) -> Vec<String> {
    let mut contexts = Vec::new();
    if example.starts_with("oval") {
        contexts.push(example.to_owned());
    }
    for declared in [String::new(), declarations(FRANKLIN)] {
        let in_ballot = |opening: &str, indent: usize| {
            format!(
                "{GUIDE}{declared}ballot @zb \"Z\"\n{opening}{}",
                indented(example, indent)
            )
        };
        contexts.extend([
            format!("oval 1\n{declared}{example}{ROOT_BALLOT}"),
            format!("{GUIDE}{declared}{example}{ROOT_BALLOT}"),
            format!("{GUIDE}{declared}{example}{}", used_elsewhere(example)),
            in_ballot("", 2),
            in_ballot("  section \"Z\"\n", 4),
            in_ballot("  contest @zc \"Z\" single\n", 4),
            in_ballot("  contest @zc \"Z\" ranked\n", 4),
            format!(
                "{GUIDE}{declared}party @zp \"Z\"\n{}{ROOT_BALLOT}",
                indented(example, 2)
            ),
        ]);
    }
    contexts
}

fn is_clean(report: &Report) -> bool {
    report
        .diagnostics()
        .iter()
        .all(|diagnostic| match diagnostic.code {
            DiagnosticCode::Lexical(_) => false,
            DiagnosticCode::Syntax(kind) => kind == SyntaxErrorKind::MissingBlock,
            DiagnosticCode::Validation(_) => true,
        })
}

fn reports(report: &Report, identifier: &str) -> bool {
    report
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code.spec_id() == identifier)
}

#[test]
fn validation_matches_the_examples_of_each_error_and_warning() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for example in examples(SPEC) {
        let Some((heading, identifier)) = entry(SPEC, example.line) else {
            continue;
        };
        checked += 1;
        let matched = contexts(&example.content).iter().any(|text| {
            let report = check(text).expect("the example is accepted");
            is_clean(&report) && reports(&report, &identifier) == example.counter
        });
        if !matched {
            let kind = if example.counter {
                "counter-example does not report"
            } else {
                "example reports"
            };
            let document = format!("{GUIDE}{}{ROOT_BALLOT}", example.content);
            let report = check(&document).expect("the example is accepted");
            let found: Vec<_> = report
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.code.spec_id())
                .collect();
            failures.push(format!(
                "SPEC.md line {} ({heading}): the {kind} {identifier} in every context\n{}\
                 as a document: {found:?}",
                example.line, example.content,
            ));
        }
    }
    println!("checked {checked} examples of validation errors and warnings");
    assert!(checked > 0, "no examples of validation errors were checked");
    assert!(
        failures.is_empty(),
        "{} of {checked} examples do not match their entry:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}
