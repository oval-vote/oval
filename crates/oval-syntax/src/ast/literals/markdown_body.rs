use crate::ast::{AstToken, ast_token};

ast_token! {
    /// The body of a Markdown field, such as the indented lines below `note:`.
    ///
    /// Its [`text`](AstToken::text) is the body exactly as written, from the first line's
    /// indentation through the last line's terminator. Its [`value`](MarkdownBody::value) is the
    /// Markdown those lines hold.
    MarkdownBody(MarkdownBlock)
}

impl MarkdownBody {
    /// Returns the body's Markdown, with the indentation its lines share removed, each line
    /// terminator written as `\n`, and no terminator after the last line.
    ///
    /// The shared indentation is the fewest spaces before any line with content. Lines keep any
    /// indentation beyond it, and a line of only spaces becomes empty. A tab is not indentation
    /// but part of the line, since a body is Markdown and its characters are all text.
    ///
    /// ```
    /// use oval_syntax::ast::{AstToken, MarkdownBody};
    ///
    /// let result = oval_syntax::parse("note:\n    Read *both* arguments.\n\n      In full.\n")?;
    /// let body = result
    ///     .syntax()
    ///     .descendants_with_tokens()
    ///     .find_map(|element| element.into_token().and_then(MarkdownBody::cast))
    ///     .expect("the note has a body");
    /// assert_eq!(body.value(), "Read *both* arguments.\n\n  In full.");
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    pub fn value(&self) -> String {
        let text = self.text().replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = text
            .strip_suffix('\n')
            .unwrap_or(&text)
            .split('\n')
            .collect();
        let indentation = lines
            .iter()
            .filter(|line| !is_blank(line))
            .map(|line| line.len() - line.trim_start_matches(' ').len())
            .min()
            .unwrap_or(0);
        let lines: Vec<&str> = lines
            .into_iter()
            .map(|line| {
                if is_blank(line) {
                    ""
                } else {
                    &line[indentation..]
                }
            })
            .collect();
        lines.join("\n")
    }
}

fn is_blank(line: &str) -> bool {
    line.bytes().all(|byte| byte == b' ')
}

#[cfg(test)]
mod tests {
    use super::MarkdownBody;
    use crate::SyntaxKind;
    use crate::ast::AstToken;
    use crate::ast::literals::tokens;

    fn body(text: &str) -> MarkdownBody {
        tokens(text)
            .into_iter()
            .find_map(MarkdownBody::cast)
            .expect("the field has a body")
    }

    #[test]
    fn a_body_loses_its_indentation() {
        assert_eq!(body("note:\n  Always check.\n").value(), "Always check.");
        assert_eq!(body("note:\n      Deep.\n").value(), "Deep.");
    }

    #[test]
    fn a_body_keeps_indentation_beyond_what_its_lines_share() {
        assert_eq!(
            body("note:\n  - One\n    - Nested\n  - Two\n").value(),
            "- One\n  - Nested\n- Two"
        );
        assert_eq!(
            body("note:\n    Code.\n  Text.\n").value(),
            "  Code.\nText."
        );
    }

    #[test]
    fn blank_lines_inside_a_body_become_empty() {
        assert_eq!(body("note:\n  A\n\n  B\n").value(), "A\n\nB");
        assert_eq!(body("note:\n  A\n      \n  B\n").value(), "A\n\nB");
        assert_eq!(body("note:\n  A\n \n  B\n").value(), "A\n\nB");
    }

    #[test]
    fn every_terminator_becomes_a_newline() {
        assert_eq!(body("note:\r\n  A\r\n  B\r\n").value(), "A\nB");
        assert_eq!(body("note:\r  A\r  B\r").value(), "A\nB");
        assert_eq!(body("note:\n  A\r\n  B\r  C\n").value(), "A\nB\nC");
    }

    #[test]
    fn a_body_at_the_end_of_input_has_no_terminator_to_drop() {
        assert_eq!(body("note:\n  A\n  B").value(), "A\nB");
    }

    #[test]
    fn a_tab_is_text_rather_than_indentation() {
        assert_eq!(body("note:\n  \tA\n  B\n").value(), "\tA\nB");
    }

    #[test]
    fn a_body_below_an_indented_field_loses_all_its_indentation() {
        assert_eq!(
            body("party @d \"D\"\n  note:\n      Deep.\n").value(),
            "Deep."
        );
    }

    #[test]
    fn a_body_keeps_its_text_as_written() {
        assert_eq!(body("note:\n  A\r\n\n  B\n").text(), "  A\r\n\n  B\n");
    }

    #[test]
    fn only_a_markdown_block_token_is_a_markdown_body() {
        assert!(MarkdownBody::can_cast(SyntaxKind::MarkdownBlock));
        assert!(!MarkdownBody::can_cast(SyntaxKind::String));
        let word = tokens("note:\n  A\n")
            .into_iter()
            .find(|token| token.kind() == SyntaxKind::Word)
            .expect("the field has a keyword");
        assert_eq!(MarkdownBody::cast(word), None);
    }
}
