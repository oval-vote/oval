//! Checks the lexer against the token sequences and errors in the specification's examples.
//!
//! TEMPORARY: this test and its copy of the specification in `fixtures/SPEC.md` exist to check the
//! rewrite during development. Delete both before the rewrite is merged. Refresh the copy with
//! `cp ../oval-spec/SPEC.md crates/oval-syntax/tests/fixtures/SPEC.md`.

use oval_syntax::{TokenKind, lex};

const SPEC: &str = include_str!("fixtures/SPEC.md");

// Token kinds the lexer does not produce yet. Examples that use any of them are skipped. Remove a
// kind from this list in the step that starts lexing it.
const NOT_YET_LEXED: &[&str] = &["Url", "Indent", "Dedent", "MarkdownBlock"];

// Errors the lexer does not report yet. Examples that expect any of them are skipped. Remove an
// identifier from this list in the step that starts reporting it.
const NOT_YET_REPORTED: &[&str] = &["malformed-url", "inconsistent-dedent"];

struct Block<'a> {
    info: &'a str,
    content: String,
    line: usize,
}

// Every fenced code block in `spec`, with its info string and the line its opening fence is on.
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

// Splits a line such as `Word «subtitle»` into its name and its decoded text, if it has any.
fn parse_line(line: &str) -> (&str, Option<String>) {
    match line.split_once(" «") {
        Some((name, rest)) => {
            let text = rest.strip_suffix('»').expect("the text ends with »");
            (name, Some(decode(text)))
        }
        None => (line, None),
    }
}

// Turns the specification's visible symbols back into the characters they stand for.
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
