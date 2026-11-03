//! Formats Oval documents.

use oval_syntax::parser::{ParseResult, parse};
use oval_syntax::{SyntaxElement, SyntaxKind, SyntaxToken, WalkEvent};

const INDENT: &str = "  ";

/// Formats `source`, or returns the parse result when the document has syntax errors.
pub fn format(source: &str) -> Result<String, ParseResult> {
    let result = parse(source);
    if !result.errors().is_empty() {
        return Err(result);
    }
    let mut formatter = Formatter {
        out: String::with_capacity(source.len()),
        depth: 0,
        at_line_start: true,
        pending: Vec::new(),
        line_has_comment: false,
    };
    for event in result.syntax().preorder_with_tokens() {
        match event {
            WalkEvent::Enter(SyntaxElement::Node(node)) if node.kind() == SyntaxKind::Block => {
                formatter.depth += 1;
            }
            WalkEvent::Leave(SyntaxElement::Node(node)) if node.kind() == SyntaxKind::Block => {
                formatter.depth -= 1;
            }
            WalkEvent::Enter(SyntaxElement::Token(token)) => formatter.token(&token),
            _ => {}
        }
    }
    Ok(formatter.finish())
}

enum Pending {
    Comment(String),
    Blank,
}

struct Formatter {
    out: String,
    depth: usize,
    at_line_start: bool,
    pending: Vec<Pending>,
    line_has_comment: bool,
}

impl Formatter {
    fn token(&mut self, token: &SyntaxToken) {
        match token.kind() {
            SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Dedent => {}
            SyntaxKind::Comment if self.at_line_start => {
                self.pending.push(Pending::Comment(token.text().to_owned()));
                self.line_has_comment = true;
            }
            SyntaxKind::Comment => {
                self.out.push(' ');
                self.out.push_str(token.text());
            }
            SyntaxKind::BlankLine if self.line_has_comment => self.line_has_comment = false,
            SyntaxKind::BlankLine if !self.pending.is_empty() => self.pending.push(Pending::Blank),
            SyntaxKind::Newline | SyntaxKind::BlankLine => {
                if !token.text().is_empty() {
                    self.out.push('\n');
                }
                self.at_line_start = true;
            }
            SyntaxKind::MarkdownBlock => {
                self.flush(self.depth);
                self.markdown(token.text());
            }
            kind => {
                if self.at_line_start {
                    self.flush(self.depth);
                    self.indent(self.depth);
                } else if !matches!(kind, SyntaxKind::Colon | SyntaxKind::Comma) {
                    self.out.push(' ');
                }
                self.out.push_str(token.text());
                self.at_line_start = false;
            }
        }
    }

    fn flush(&mut self, depth: usize) {
        for pending in std::mem::take(&mut self.pending) {
            match pending {
                Pending::Comment(text) => {
                    self.indent(depth);
                    self.out.push_str(&text);
                    self.out.push('\n');
                }
                Pending::Blank => self.out.push('\n'),
            }
        }
    }

    fn markdown(&mut self, text: &str) {
        let lines: Vec<&str> = split_lines(text);
        let common = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start_matches(' ').len())
            .min()
            .unwrap_or(0);
        for line in lines {
            if !line.trim().is_empty() {
                self.indent(self.depth + 1);
                self.out.push_str(&line[common..]);
            }
            self.out.push('\n');
        }
        self.at_line_start = true;
    }

    fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.out.push_str(INDENT);
        }
    }

    fn finish(mut self) -> String {
        self.flush(0);
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
    }
}

fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(['\n', '\r']) {
        lines.push(&rest[..i]);
        let after = if rest[i..].starts_with("\r\n") {
            i + 2
        } else {
            i + 1
        };
        rest = &rest[after..];
    }
    if !rest.is_empty() {
        lines.push(rest);
    }
    lines
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use oval_syntax::SyntaxElement;
    use oval_syntax::SyntaxKind::{Dedent, Indent, MarkdownBlock, Newline};
    use oval_syntax::ast::{AstNode, MarkdownField};
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

    fn markdown_values(source: &str) -> Vec<String> {
        parse(source)
            .syntax()
            .descendants()
            .filter_map(MarkdownField::cast)
            .filter_map(|f| f.value())
            .collect()
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
        assert_eq!(
            markdown_values(&formatted),
            markdown_values(source),
            "same markdown"
        );
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
    fn trailing_spaces_are_removed_except_inside_markdown_where_they_may_be_hard_breaks() {
        check(
            "guide \"G\"   \nnote:  \n  Body.  \n\n   \n  More.\n",
            "guide \"G\"\nnote:\n  Body.  \n\n\n  More.\n",
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

    #[test]
    fn indentation_is_two_spaces_per_block() {
        check(
            "ballot @b \"B\"\n    section \"S\"\n            contest @c \"C\" single\n                        campaign @a \"A\"\n                                designation \"d\"\n                        recommend\n                                support @a\n",
            "ballot @b \"B\"\n  section \"S\"\n    contest @c \"C\" single\n      campaign @a \"A\"\n        designation \"d\"\n      recommend\n        support @a\n",
        );
        check(
            "ballot @b \"B\"\n ballot @c \"C\"\n   measure @m \"M\"\n     recommend yes\n",
            "ballot @b \"B\"\n  ballot @c \"C\"\n    measure @m \"M\"\n      recommend yes\n",
        );
    }

    #[test]
    fn markdown_bodies_move_with_their_field_and_keep_relative_indentation() {
        check(
            "ballot @b \"B\"\n    note:\n            First.\n\n            - item\n              continued\n            - next\n",
            "ballot @b \"B\"\n  note:\n    First.\n\n    - item\n      continued\n    - next\n",
        );
        check("note:\n      deep\n    less\n", "note:\n    deep\n  less\n");
    }

    #[test]
    fn tokens_are_separated_by_one_space_with_punctuation_attached() {
        check(
            "ballot   @b    \"B\"\n  contest @c  \"C\"   ranked   max   5\n    campaign    incumbent   @a   \"A\"\n    recommend\n      support   @a\n  measure @m \"M\"\n    supporters \"a\" ,\"b\"   ,  \"c\"\n    note :\n      x\n    recommend   yes\n",
            "ballot @b \"B\"\n  contest @c \"C\" ranked max 5\n    campaign incumbent @a \"A\"\n    recommend\n      support @a\n  measure @m \"M\"\n    supporters \"a\", \"b\", \"c\"\n    note:\n      x\n    recommend yes\n",
        );
    }

    #[test]
    fn comment_lines_take_the_indentation_of_their_block() {
        check(
            "ballot @b \"B\"\n# about the measure\n        measure @m \"M\"\n            recommend yes    # trailing\n",
            "ballot @b \"B\"\n  # about the measure\n  measure @m \"M\"\n    recommend yes # trailing\n",
        );
    }

    #[test]
    fn comments_take_the_indentation_of_the_line_that_follows() {
        check(
            "ballot @a \"A\"\n  measure @m \"M\"\n    recommend yes\n        # between roots\n\n        # still between\nballot @b \"B\"\n# first in block\n  measure @n \"N\"\n    recommend no\n    # end of file\n",
            "ballot @a \"A\"\n  measure @m \"M\"\n    recommend yes\n# between roots\n\n# still between\nballot @b \"B\"\n  # first in block\n  measure @n \"N\"\n    recommend no\n# end of file\n",
        );
    }
}
