//! Checks the lexer and parser against the specification's examples.
//!
//! TEMPORARY: these tests and their copies of the specification in `fixtures/SPEC.md` and of its
//! example guide in `fixtures/franklin.oval` exist to check the rewrite during development. Delete
//! them before the rewrite is merged. Refresh the copies with
//! `cp ../oval-spec/SPEC.md ../oval-spec/examples/franklin.oval crates/oval-syntax/tests/fixtures/`.

use oval_syntax::{ParseResult, TokenKind, lex, parse};

const SPEC: &str = include_str!("fixtures/SPEC.md");

const FRANKLIN: &str = include_str!("fixtures/franklin.oval");

const CONTEXTS: &[(&str, &str, usize)] = &[
    ("document", "", 0),
    ("ballot", "ballot @x \"X\"\n", 2),
    ("section", "ballot @x \"X\"\n  section \"S\"\n", 4),
    ("contest", "ballot @x \"X\"\n  contest @y \"Y\" single\n", 4),
    ("measure", "ballot @x \"X\"\n  measure @y \"Y\"\n", 4),
    ("party", "party @x \"X\"\n", 2),
    ("candidate", "candidate @x \"X\"\n", 2),
    (
        "campaign",
        "ballot @x \"X\"\n  contest @y \"Y\" single\n    campaign\n",
        6,
    ),
];
const NOT_YET_LEXED: &[&str] = &[];

const NOT_YET_REPORTED: &[&str] = &[];

struct Block<'a> {
    info: &'a str,
    content: String,
    line: usize,
}

fn blocks(spec: &str) -> Vec<Block<'_>> {
    let mut blocks = Vec::new();
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
        blocks.push(Block {
            info,
            content,
            line: index + 1,
        });
    }
    blocks
}

fn parse_line(line: &str) -> (&str, Option<String>) {
    match line.split_once(" «") {
        Some((name, rest)) => {
            let text = rest.strip_suffix('»').expect("the text ends with »");
            (name, Some(decode(text)))
        }
        None => (line, None),
    }
}

fn decode(text: &str) -> String {
    text.replace('␣', " ").replace('⇥', "\t").replace('↵', "\n")
}

fn token_kind(name: &str, text: &str) -> TokenKind {
    match (name, text) {
        ("Word", _) => TokenKind::Word,
        ("String", _) => TokenKind::String,
        ("Integer", _) => TokenKind::Integer,
        ("Date", _) => TokenKind::Date,
        ("Url", _) => TokenKind::Url,
        ("Punctuator", "@") => TokenKind::AtSign,
        ("Punctuator", ":") => TokenKind::Colon,
        ("Punctuator", ",") => TokenKind::Comma,
        ("MarkdownBlock", _) => TokenKind::MarkdownBlock,
        ("Whitespace", _) => TokenKind::Whitespace,
        ("Comment", _) => TokenKind::Comment,
        ("BlankLine", _) => TokenKind::BlankLine,
        ("ByteOrderMark", _) => TokenKind::ByteOrderMark,
        ("Newline", _) => TokenKind::Newline,
        ("Indent", _) => TokenKind::Indent,
        ("Dedent", _) => TokenKind::Dedent,
        ("Error", _) => TokenKind::Error,
        _ => panic!("the specification uses an unknown token: {name} «{text}»"),
    }
}

#[test]
fn lexer_matches_the_specification_examples() {
    let blocks = blocks(SPEC);
    let mut checked = 0;
    let mut skipped = Vec::new();
    let mut failures = Vec::new();

    for (index, block) in blocks.iter().enumerate() {
        if block.info != "tokens" {
            continue;
        }
        let errors_block = blocks.get(index + 1).filter(|next| next.info == "errors");
        let uses_tokens_not_lexed = block
            .content
            .lines()
            .any(|line| NOT_YET_LEXED.contains(&parse_line(line).0));
        let expects_errors_not_reported = errors_block.is_some_and(|errors| {
            errors
                .content
                .lines()
                .any(|line| NOT_YET_REPORTED.contains(&parse_line(line).0))
        });
        if uses_tokens_not_lexed || expects_errors_not_reported {
            skipped.push(block.line);
            continue;
        }

        let source = &blocks[..index]
            .iter()
            .rev()
            .find(|block| block.info.starts_with("oval"))
            .expect("an example comes before each token sequence")
            .content;
        let expected_errors: Vec<(String, String)> = errors_block
            .map(|next| {
                next.content
                    .lines()
                    .map(|line| {
                        let (identifier, text) = parse_line(line);
                        (identifier.to_owned(), text.unwrap_or_default())
                    })
                    .collect()
            })
            .unwrap_or_default();
        let expected_tokens: Vec<(TokenKind, String)> = block
            .content
            .lines()
            .map(|line| {
                let (name, text) = parse_line(line);
                let text = text.unwrap_or_default();
                (token_kind(name, &text), text)
            })
            .collect();

        let result = lex(source).expect("the example is accepted");
        let mut pos = 0;
        let actual_tokens: Vec<(TokenKind, String)> = result
            .tokens
            .iter()
            .map(|token| {
                let len = token.len as usize;
                let text = source[pos..pos + len].to_owned();
                pos += len;
                (token.kind, text)
            })
            .collect();
        let actual_errors: Vec<(String, String)> = result
            .errors
            .iter()
            .map(|error| {
                let start = error.offset as usize;
                let text = source[start..start + error.len as usize].to_owned();
                (error.kind.spec_id().to_owned(), text)
            })
            .collect();

        checked += 1;
        if actual_tokens != expected_tokens || actual_errors != expected_errors {
            failures.push(format!(
                "SPEC.md line {}, example:\n{source}\
                 expected tokens: {expected_tokens:?}\n  actual tokens: {actual_tokens:?}\n\
                 expected errors: {expected_errors:?}\n  actual errors: {actual_errors:?}",
                block.line,
            ));
        }
    }

    println!(
        "checked {checked} examples; skipped {} that need work not done yet (lines {skipped:?})",
        skipped.len(),
    );
    assert!(checked > 0, "no specification examples were checked");
    assert!(
        failures.is_empty(),
        "{} of {checked} specification examples failed:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}

fn heading_before(spec: &str, line: usize) -> (&str, &str) {
    let mut section = "";
    let mut heading = "";
    for text in spec.lines().take(line) {
        if let Some(rest) = text.strip_prefix("### ") {
            section = rest;
        } else if let Some(rest) = text.strip_prefix("#### ") {
            heading = rest;
        }
    }
    (section, heading)
}

fn in_context(example: &str, prefix: &str, indent: usize) -> String {
    let mut text = prefix.to_owned();
    for line in example.lines() {
        if !line.is_empty() {
            text.push_str(&" ".repeat(indent));
        }
        text.push_str(line);
        text.push('\n');
    }
    text
}

fn lexical_errors_are_elisions(text: &str, result: &ParseResult) -> bool {
    result.lexical_errors.iter().all(|error| {
        let start = error.offset as usize;
        error.kind.spec_id() == "unexpected-character"
            && &text[start..start + error.len as usize] == "."
    })
}

fn describe(text: &str, result: &ParseResult) -> String {
    let lexical: Vec<_> = result
        .lexical_errors
        .iter()
        .map(ToString::to_string)
        .collect();
    let syntax: Vec<_> = result
        .syntax_errors
        .iter()
        .map(ToString::to_string)
        .collect();
    format!("{text}  lexical errors: {lexical:?}\n  syntax errors: {syntax:?}")
}

#[test]
fn parser_accepts_the_specification_examples() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for block in blocks(SPEC) {
        if block.info != "oval example" {
            continue;
        }
        checked += 1;
        let accepted = CONTEXTS.iter().any(|(_, prefix, indent)| {
            let text = in_context(&block.content, prefix, *indent);
            let result = parse(&text).expect("the example is accepted");
            lexical_errors_are_elisions(&text, &result)
                && result
                    .syntax_errors
                    .iter()
                    .all(|error| error.spec_id() == "missing-block")
        });
        if !accepted {
            let result = parse(&block.content).expect("the example is accepted");
            failures.push(format!(
                "SPEC.md line {}, as a document:\n{}",
                block.line,
                describe(&block.content, &result),
            ));
        }
    }
    println!("parsed {checked} examples");
    assert!(checked > 0, "no specification examples were parsed");
    assert!(
        failures.is_empty(),
        "{} of {checked} specification examples have errors in every context:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}

#[test]
fn parser_reports_the_error_of_each_syntax_counter_example() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for block in blocks(SPEC) {
        let (section, heading) = heading_before(SPEC, block.line);
        if block.info != "oval counter-example" || !section.starts_with("3.2.") {
            continue;
        }
        checked += 1;
        let expected = heading.to_lowercase().replace(' ', "-");
        let reported = CONTEXTS.iter().any(|(_, prefix, indent)| {
            let text = in_context(&block.content, prefix, *indent);
            let result = parse(&text).expect("the counter-example is accepted");
            let ids: Vec<&str> = result
                .syntax_errors
                .iter()
                .map(oval_syntax::SyntaxError::spec_id)
                .filter(|id| *id != "missing-block" || expected == "missing-block")
                .collect();
            let fits_context = if expected == "unexpected-entry" {
                ids.iter().all(|id| *id == expected)
            } else {
                !ids.contains(&"unexpected-entry")
            };
            result.lexical_errors.is_empty() && ids.contains(&expected.as_str()) && fits_context
        });
        if !reported {
            let result = parse(&block.content).expect("the counter-example is accepted");
            failures.push(format!(
                "SPEC.md line {}, expecting {expected}, as a document:\n{}",
                block.line,
                describe(&block.content, &result),
            ));
        }
    }
    println!("checked {checked} syntax counter-examples");
    assert!(checked > 0, "no syntax counter-examples were checked");
    assert!(
        failures.is_empty(),
        "{} of {checked} syntax counter-examples do not report their error:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}

#[test]
fn parser_accepts_the_example_guide() {
    let result = parse(FRANKLIN).expect("the example guide is accepted");
    assert!(
        result.lexical_errors.is_empty() && result.syntax_errors.is_empty(),
        "{}",
        describe("", &result),
    );
}
