use crate::TextSize;
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
        let lines = lines(self.text());
        let indentation = indentation(&lines);
        lines
            .iter()
            .map(|(_, line)| removed(line, indentation))
            .map(|(_, rest)| rest)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns where a byte offset into the body's [`value`](Self::value) falls in the document.
    ///
    /// An offset within a line maps to the same character as written, past the indentation the
    /// value removed. An offset at the end of a line maps to where that line's terminator begins,
    /// and one at or past the end of the value maps to the end of the last line.
    ///
    /// ```
    /// use oval_syntax::TextSize;
    /// use oval_syntax::ast::{AstToken, MarkdownBody};
    ///
    /// let result = oval_syntax::parse("note:\n  One\n  *Two*\n")?;
    /// let body = result
    ///     .syntax()
    ///     .descendants_with_tokens()
    ///     .find_map(|element| element.into_token().and_then(MarkdownBody::cast))
    ///     .expect("the note has a body");
    /// assert_eq!(body.value(), "One\n*Two*");
    /// assert_eq!(body.source_offset(4), TextSize::from(14));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    pub fn source_offset(&self, offset: usize) -> TextSize {
        let text = self.text();
        let start = self.syntax().text_range().start();
        let lines = lines(text);
        let indentation = indentation(&lines);
        let mut line_offset = 0;
        let mut last_end = 0;
        for (line_start, line) in &lines {
            let (indent, rest) = removed(line, indentation);
            if offset <= line_offset + rest.len() {
                let local = line_start + indent + (offset - line_offset);
                return start + TextSize::of(&text[..local]);
            }
            line_offset += rest.len() + 1;
            last_end = line_start + line.len();
        }
        start + TextSize::of(&text[..last_end])
    }
}

fn lines(text: &str) -> Vec<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut position = 0;
    while position < bytes.len() {
        match bytes[position] {
            b'\n' => {
                lines.push((start, &text[start..position]));
                position += 1;
                start = position;
            }
            b'\r' => {
                lines.push((start, &text[start..position]));
                position += if bytes.get(position + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                start = position;
            }
            _ => position += 1,
        }
    }
    if start < text.len() {
        lines.push((start, &text[start..]));
    }
    lines
}

fn indentation(lines: &[(usize, &str)]) -> usize {
    lines
        .iter()
        .filter(|(_, line)| !is_blank(line))
        .map(|(_, line)| line.len() - line.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0)
}

fn removed(line: &str, indentation: usize) -> (usize, &str) {
    if is_blank(line) {
        (line.len(), "")
    } else {
        (indentation, &line[indentation..])
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

    fn offsets(text: &str, offsets: &[usize]) -> Vec<u32> {
        let body = body(text);
        offsets
            .iter()
            .map(|offset| body.source_offset(*offset).into())
            .collect()
    }

    #[test]
    fn an_offset_maps_past_the_indentation_the_value_removed() {
        let text = "note:\n  A\n    Deep\n  B\n";
        assert_eq!(body(text).value(), "A\n  Deep\nB");
        assert_eq!(offsets(text, &[0, 1, 2, 4, 8, 9]), [8, 9, 12, 14, 18, 21]);
    }

    #[test]
    fn a_blank_line_maps_to_its_terminator() {
        let text = "note:\n  A\n   \n  B\n";
        assert_eq!(body(text).value(), "A\n\nB");
        assert_eq!(offsets(text, &[1, 2, 3]), [9, 13, 16]);
    }

    #[test]
    fn an_offset_skips_the_extra_byte_of_a_crlf_terminator() {
        let text = "note:\r\n  A\r\n  B\r\n";
        assert_eq!(body(text).value(), "A\nB");
        assert_eq!(offsets(text, &[0, 1, 2, 3]), [9, 10, 14, 15]);
    }

    #[test]
    fn an_offset_at_or_past_the_end_maps_to_the_end_of_the_last_line() {
        let text = "party @d \"D\"\n  note:\n      Deep.\n";
        assert_eq!(offsets(text, &[5, 99]), [32, 32]);
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
