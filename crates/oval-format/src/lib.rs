//! Formats Oval documents.

use oval_syntax::parser::{ParseResult, parse};
use oval_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

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
    let trailing = formatter.node(&result.syntax());
    formatter.tokens(&trailing);
    Ok(formatter.finish())
}

struct Entry {
    leading: Vec<SyntaxToken>,
    node: SyntaxNode,
}

fn group(block: &SyntaxNode, kind: SyntaxKind) -> u8 {
    let parent = block.parent().map(|p| p.kind());
    match kind {
        SyntaxKind::Member | SyntaxKind::Support => 0,
        SyntaxKind::Oppose
        | SyntaxKind::Campaign
        | SyntaxKind::Section
        | SyntaxKind::Contest
        | SyntaxKind::Measure
        | SyntaxKind::Ballot => 1,
        SyntaxKind::ContestRecommend | SyntaxKind::MeasureRecommend => 2,
        _ if parent == Some(SyntaxKind::Campaign) => 1,
        _ if parent == Some(SyntaxKind::ContestRecommend) => 2,
        _ => 0,
    }
}

fn trailing_trivia(node: &SyntaxNode) -> Vec<SyntaxToken> {
    let Some(SyntaxElement::Node(block)) = node.last_child_or_token() else {
        return Vec::new();
    };
    if block.kind() != SyntaxKind::Block {
        return Vec::new();
    }
    let mut after = Vec::new();
    let mut last = None;
    for element in block.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => {
                after.clear();
                last = Some(child);
            }
            SyntaxElement::Token(token) if token.kind().is_trivia() => after.push(token),
            SyntaxElement::Token(_) => {}
        }
    }
    let mut out = last.as_ref().map(trailing_trivia).unwrap_or_default();
    out.extend(after);
    out
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
    fn node(&mut self, node: &SyntaxNode) -> Vec<SyntaxToken> {
        let mut carried: Vec<SyntaxToken> = Vec::new();
        for element in node.children_with_tokens() {
            match element {
                SyntaxElement::Token(token) if token.kind().is_trivia() => carried.push(token),
                SyntaxElement::Token(token) => {
                    self.tokens(&std::mem::take(&mut carried));
                    self.token(&token);
                }
                SyntaxElement::Node(child) if child.kind() == SyntaxKind::Block => {
                    carried = self.block(&child, std::mem::take(&mut carried));
                }
                SyntaxElement::Node(child) => {
                    self.tokens(&std::mem::take(&mut carried));
                    carried = self.node(&child);
                }
            }
        }
        carried
    }

    fn tokens(&mut self, tokens: &[SyntaxToken]) {
        for token in tokens {
            self.token(token);
        }
    }

    fn block(&mut self, block: &SyntaxNode, before: Vec<SyntaxToken>) -> Vec<SyntaxToken> {
        self.depth += 1;
        let mut entries: Vec<Entry> = Vec::new();
        let mut leading = before;
        for element in block.children_with_tokens() {
            match element {
                SyntaxElement::Token(token) if token.kind().is_trivia() => leading.push(token),
                SyntaxElement::Token(_) => {}
                SyntaxElement::Node(node) => entries.push(Entry {
                    leading: std::mem::take(&mut leading),
                    node,
                }),
            }
        }
        let mut trailing = Vec::new();
        for i in 0..entries.len() {
            let moved = trailing_trivia(&entries[i].node);
            match entries.get_mut(i + 1) {
                Some(next) => {
                    next.leading.splice(0..0, moved);
                }
                None => trailing = moved,
            }
        }
        trailing.extend(leading);
        entries.sort_by_key(|entry| group(block, entry.node.kind()));
        for entry in entries {
            self.tokens(&entry.leading);
            self.node(&entry.node);
        }
        self.depth -= 1;
        trailing
    }

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
            SyntaxKind::BlankLine => self.blank(),
            SyntaxKind::Newline => {
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
                Pending::Blank => self.blank(),
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

    fn blank(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with("\n\n") {
            self.out.push('\n');
        }
        self.at_line_start = true;
    }

    fn finish(mut self) -> String {
        self.flush(0);
        while self.out.ends_with("\n\n") {
            self.out.pop();
        }
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
    use oval_syntax::SyntaxElement;
    use oval_syntax::SyntaxKind::{Dedent, Indent, MarkdownBlock, Newline};
    use oval_syntax::ast::{AstNode, MarkdownField};
    use oval_syntax::parser::parse;

    use super::format;

    const FIXTURE: &str = include_str!("../../oval-syntax/tests/fixtures/franklin.oval");

    fn tokens(source: &str) -> Vec<(oval_syntax::SyntaxKind, String)> {
        let mut tokens: Vec<_> = parse(source)
            .syntax()
            .descendants_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|t| {
                !t.kind().is_trivia()
                    && !matches!(t.kind(), Newline | Indent | Dedent | MarkdownBlock)
            })
            .map(|t| (t.kind(), t.text().to_owned()))
            .collect();
        tokens.sort();
        tokens
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
        assert_eq!(tokens(&formatted), tokens(source), "same tokens");
        if let Ok(compiled) = oval_core::compile(source) {
            assert_eq!(
                oval_core::compile(&formatted).ok(),
                Some(compiled),
                "same compiled model"
            );
        }
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
    fn comments_and_single_blank_lines_survive() {
        let source = "# lead\n\nguide \"G\" # trail\n\n# tail\n";
        check(source, source);
    }

    #[test]
    fn runs_of_blank_lines_collapse_to_one() {
        check(
            "guide \"G\"\n\n\n\nelection \"E\"\n\n\n# c\n\n\n\n# d\ndate 2026-11-03\n",
            "guide \"G\"\n\nelection \"E\"\n\n# c\n\n# d\ndate 2026-11-03\n",
        );
    }

    #[test]
    fn blank_lines_at_the_start_and_end_are_removed() {
        check("\n\n\nguide \"G\"\n\n\n\n", "guide \"G\"\n");
        check("\n   \n# only a comment\n\n", "# only a comment\n");
        check("\n\n", "");
    }

    #[test]
    fn blank_lines_inside_markdown_are_content_and_are_kept() {
        let source = "note:\n  One.\n\n\n\n  Two.\n";
        check(source, source);
    }

    #[test]
    fn blank_lines_around_blocks_are_the_authors_choice() {
        let source = "ballot @b \"B\"\n\n  section \"S\"\n    measure @m \"M\"\n\n      recommend yes\n\n  measure @n \"N\"\n    recommend no\n";
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

    #[test]
    fn block_entries_follow_the_grammars_order() {
        check(
            "oval 0.1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  contest @c \"C\" single\n    recommend\n      justification:\n        Why.\n      oppose @y\n      support @x\n    campaign @x \"X\"\n    subtitle \"S\"\n    campaign @y \"Y\"\n    tabulation \"T\"\n  eligibility \"E\"\n",
            "oval 0.1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  eligibility \"E\"\n  contest @c \"C\" single\n    subtitle \"S\"\n    tabulation \"T\"\n    campaign @x \"X\"\n    campaign @y \"Y\"\n    recommend\n      support @x\n      oppose @y\n      justification:\n        Why.\n",
        );
    }

    #[test]
    fn campaign_members_come_before_campaign_fields() {
        check(
            "ballot @b \"B\"\n  contest @c \"C\" single\n    campaign\n      website https://t.example\n      @p \"P\" as \"President\"\n      note:\n        Ticket.\n      @v \"V\" as \"Vice President\"\n    recommend\n      support @p\n",
            "ballot @b \"B\"\n  contest @c \"C\" single\n    campaign\n      @p \"P\" as \"President\"\n      @v \"V\" as \"Vice President\"\n      website https://t.example\n      note:\n        Ticket.\n    recommend\n      support @p\n",
        );
    }

    #[test]
    fn measure_recommendation_moves_after_fields() {
        check(
            "ballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n    subtitle \"S\"\n    supporters none\n",
            "ballot @b \"B\"\n  measure @m \"M\"\n    subtitle \"S\"\n    supporters none\n    recommend yes\n",
        );
    }

    #[test]
    fn comments_and_blank_lines_travel_with_the_entry_they_precede() {
        check(
            "ballot @b \"B\"\n  contest @c \"C\" single\n    # the advice\n    recommend\n      support @x\n\n    # the field\n    subtitle \"S\"\n\n    # the only campaign\n    campaign @x \"X\"\n    # end of contest\n",
            "ballot @b \"B\"\n  contest @c \"C\" single\n\n    # the field\n    subtitle \"S\"\n\n    # the only campaign\n    campaign @x \"X\"\n    # the advice\n    recommend\n      support @x\n# end of contest\n",
        );
    }

    #[test]
    fn entries_already_in_order_are_untouched() {
        let source = "ballot @b \"B\"\n  note:\n    x\n  section \"S\"\n    contest @c \"C\" multiple 2\n      subtitle \"S\"\n      campaign incumbent @a \"A\"\n        designation \"d\"\n        party @dem \"D\"\n      campaign @z \"Z\"\n      recommend\n        support @a @z\n        justification:\n          Both.\n";
        check(source, source);
    }
}
