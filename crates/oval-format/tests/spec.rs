//! Checks the formatter against the examples of the specification's formatting rules and its
//! example guide.
//!
//! TEMPORARY: these tests read the copies of the specification and its example guide in
//! `crates/oval-syntax/tests/fixtures`, and exist to check the rewrite during development. Delete
//! them, and the `oval-core` dev-dependency they need, before the rewrite is merged.

use oval_format::format;

const SPEC: &str = include_str!("../../oval-syntax/tests/fixtures/SPEC.md");

const FRANKLIN: &str = include_str!("../../oval-syntax/tests/fixtures/franklin.oval");

const GUIDE: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\n";

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

fn formatting_section(spec: &str) -> &str {
    let start = spec
        .find("\n## 6. Formatting")
        .expect("the specification has section 6");
    let end = spec[start + 1..]
        .find("\n## ")
        .map_or(spec.len(), |end| start + 1 + end);
    &spec[start..end]
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn contest_block(indent: usize, campaign: usize) -> String {
    let indent = " ".repeat(indent);
    format!("{indent}campaign @zc{campaign} \"C\"\n{indent}recommend no-position\n")
}

fn in_context(snippet: &str) -> String {
    let lines: Vec<&str> = snippet.lines().collect();
    let mut text = String::new();
    for (index, line) in lines.iter().enumerate() {
        let indent = indentation(line);
        if line.trim() == "..." {
            text.push_str(&contest_block(indent, index));
            continue;
        }
        text.push_str(line);
        text.push('\n');
        let has_block = lines
            .get(index + 1)
            .is_some_and(|next| indentation(next) > indent);
        if line.split_whitespace().any(|word| word == "contest") && !has_block {
            text.push_str(&contest_block(indent + 2, index));
        }
    }
    let starts_with_ballot = lines
        .first()
        .is_some_and(|first| first.split_whitespace().any(|word| word == "ballot"));
    if starts_with_ballot {
        text
    } else {
        let mut wrapped = String::from("ballot @zz \"Z\"\n");
        for line in text.lines() {
            wrapped.push_str("  ");
            wrapped.push_str(line);
            wrapped.push('\n');
        }
        wrapped
    }
}

fn compiled(text: &str) -> Option<String> {
    oval_core::check(text)
        .ok()?
        .compile()
        .map(|compiled| compiled.to_json())
}

#[test]
fn each_counter_example_of_a_formatting_rule_formats_to_the_example_after_it() {
    let examples = examples(formatting_section(SPEC));
    let mut pairs = 0;
    for pair in examples.windows(2) {
        let [counter, example] = pair else {
            unreachable!("windows of two have two examples");
        };
        if !counter.counter || example.counter {
            continue;
        }
        let expected = in_context(&example.content);
        assert_eq!(
            format(&in_context(&counter.content)).as_deref(),
            Ok(expected.as_str()),
            "the counter-example at line {} formats to the example at line {}",
            counter.line,
            example.line
        );
        assert_eq!(
            format(&expected).as_deref(),
            Ok(expected.as_str()),
            "the example at line {} is already formatted",
            example.line
        );
        let before = compiled(&format!("{GUIDE}{}", in_context(&counter.content)));
        assert!(
            before.is_some(),
            "the counter-example at line {} compiles in context",
            counter.line
        );
        assert_eq!(
            compiled(&format!("{GUIDE}{expected}")),
            before,
            "the counter-example at line {} compiles to the same output after formatting",
            counter.line
        );
        pairs += 1;
    }
    assert_eq!(
        pairs, 3,
        "section 6 has a pair for each of 6.4, 6.5, and 6.6"
    );
}

#[test]
fn the_example_guide_is_already_formatted() {
    assert_eq!(format(FRANKLIN).as_deref(), Ok(FRANKLIN));
}

#[test]
fn every_example_that_formats_formats_to_itself_and_compiles_to_the_same_output() {
    let mut formatted_count = 0;
    let mut compiled_count = 0;
    for example in examples(SPEC) {
        let Ok(formatted) = format(&example.content) else {
            continue;
        };
        assert_eq!(
            format(&formatted).as_deref(),
            Ok(formatted.as_str()),
            "the example at line {} formats to itself on a second pass",
            example.line
        );
        formatted_count += 1;
        if let Some(before) = compiled(&example.content) {
            assert_eq!(
                compiled(&formatted),
                Some(before),
                "the example at line {} compiles to the same output after formatting",
                example.line
            );
            compiled_count += 1;
        }
    }
    println!("{formatted_count} examples formatted, {compiled_count} compiled");
    assert!(formatted_count > 0);
}
