//! Formats Oval documents.

use oval_syntax::parser::{ParseResult, parse};
use oval_syntax::{SyntaxElement, SyntaxKind};

/// Formats `source`, or returns the parse result when the document has syntax errors.
pub fn format(source: &str) -> Result<String, ParseResult> {
    let result = parse(source);
    if !result.errors().is_empty() {
        return Err(result);
    }
    let mut out = String::with_capacity(source.len());
    let tokens = result
        .syntax()
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token);
    for token in tokens {
        match token.kind() {
            SyntaxKind::Newline | SyntaxKind::BlankLine => {
                trim_trailing_spaces(&mut out);
                if !token.text().is_empty() {
                    out.push('\n');
                }
            }
            SyntaxKind::MarkdownBlock => push_block(&mut out, token.text()),
            _ => out.push_str(token.text()),
        }
    }
    trim_trailing_spaces(&mut out);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn push_block(out: &mut String, text: &str) {
    let mut rest = text;
    while let Some(i) = rest.find(['\n', '\r']) {
        out.push_str(rest[..i].trim_end_matches(' '));
        out.push('\n');
        let after = if rest[i..].starts_with("\r\n") {
            i + 2
        } else {
            i + 1
        };
        rest = &rest[after..];
    }
    out.push_str(rest.trim_end_matches(' '));
}

fn trim_trailing_spaces(out: &mut String) {
    while out.ends_with(' ') {
        out.pop();
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use oval_syntax::SyntaxElement;
    use oval_syntax::SyntaxKind::{Dedent, Indent, MarkdownBlock, Newline};
    use oval_syntax::parser::parse;

    use super::format;

    const FIXTURE: &str = include_str!("../../oval-syntax/tests/fixtures/franklin.oval");

    fn shape(source: &str) -> String {
        let mut out = String::new();
        for element in parse(source).syntax().descendants_with_tokens() {
            match element {
                SyntaxElement::Node(node) => writeln!(out, "{:?}", node.kind()).unwrap(),
                SyntaxElement::Token(token) if token.kind().is_trivia() => {}
                SyntaxElement::Token(token)
                    if matches!(token.kind(), Newline | Indent | Dedent | MarkdownBlock) =>
                {
                    writeln!(out, "{:?}", token.kind()).unwrap();
                }
                SyntaxElement::Token(token) => {
                    writeln!(out, "{:?} {:?}", token.kind(), token.text()).unwrap();
                }
            }
        }
        out
    }

    fn check(source: &str, expected: &str) {
        let formatted = format(source).expect("formats");
        assert_eq!(formatted, expected);
        assert_eq!(
            format(&formatted).expect("formats again"),
            formatted,
            "idempotent"
        );
        assert_eq!(shape(&formatted), shape(source), "same tree");
    }

    #[test]
    fn the_fixture_is_already_formatted() {
        assert_eq!(format(FIXTURE).expect("formats"), FIXTURE);
    }

    #[test]
    fn line_terminators_become_newlines() {
        check(
            "guide \"G\"\r\nelection \"E\"\rdate 2026-11-03\n",
            "guide \"G\"\nelection \"E\"\ndate 2026-11-03\n",
        );
    }

    #[test]
    fn trailing_spaces_are_removed_including_inside_markdown() {
        check(
            "guide \"G\"   \nnote:  \n  Body.   \n\n   \n  More.\n",
            "guide \"G\"\nnote:\n  Body.\n\n\n  More.\n",
        );
    }

    #[test]
    fn a_final_newline_is_added() {
        check("guide \"G\"", "guide \"G\"\n");
        check("", "");
    }

    #[test]
    fn documents_with_syntax_errors_are_refused() {
        let result = format("guide yes\n").expect_err("has errors");
        assert_eq!(result.errors().len(), 1);
    }

    #[test]
    fn comments_and_blank_lines_survive() {
        let source = "# lead\n\nguide \"G\" # trail\n\n\n# tail\n";
        check(source, source);
    }
}
