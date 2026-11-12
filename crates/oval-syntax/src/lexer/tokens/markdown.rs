use crate::TokenKind;
use crate::lexer::{Lexer, is_line_terminator};

impl Lexer<'_> {
    pub(in crate::lexer) fn markdown_block(&mut self, start: usize) -> bool {
        let Some(opener) = self.body_opener else {
            return false;
        };
        let first = self.measure_line(start);
        if first.is_blank {
            return false;
        }
        self.body_opener = None;
        if first.indentation <= opener {
            return false;
        }
        let mut end = first.end;
        let mut next = first.end;
        while next < self.source.len() {
            let line = self.measure_line(next);
            if !line.is_blank && line.indentation <= opener {
                break;
            }
            next = line.end;
            if !line.is_blank {
                end = line.end;
            }
        }
        self.pos = end;
        self.push(TokenKind::MarkdownBlock, start);
        self.line_has_tokens = false;
        self.line_start = self.pos;
        true
    }

    fn measure_line(&self, start: usize) -> Line {
        let text = &self.source[start..];
        let indentation = text.bytes().take_while(|&b| b == b' ').count();
        let content_len = text.find(is_line_terminator).unwrap_or(text.len());
        let terminator_len = match &text.as_bytes()[content_len..] {
            [] => 0,
            [b'\r', b'\n', ..] => 2,
            _ => 1,
        };
        Line {
            indentation,
            is_blank: indentation == content_len,
            end: start + content_len + terminator_len,
        }
    }
}

struct Line {
    indentation: usize,
    is_blank: bool,
    end: usize,
}

#[cfg(test)]
mod tests {
    use crate::LexicalErrorKind::IllegalTab;
    use crate::TokenKind::{
        BlankLine, Colon, Comment, Dedent, Error, Indent, MarkdownBlock, Newline, Whitespace, Word,
    };
    use crate::lexer::tests::{check_lexer, error};

    #[test]
    fn a_line_ending_in_a_colon_opens_a_markdown_body() {
        check_lexer(
            "note:\n  text\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  text\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_spans_every_deeper_line() {
        check_lexer(
            "note:\n  one\n  two\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  one\n  two\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_keeps_its_own_indentation() {
        check_lexer(
            "note:\n  - one\n    more\n   two\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  - one\n    more\n   two\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_may_end_at_the_end_of_input() {
        check_lexer(
            "note:\n  text",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  text"),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_is_not_lexed() {
        check_lexer(
            "note:\n  # @x \"a\\q\tb\n  key: 12x\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  # @x \"a\\q\tb\n  key: 12x\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_deeper_comment_line_begins_a_body() {
        check_lexer(
            "note:\n  # Heading\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  # Heading\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_comment_after_the_colon_still_opens_a_body() {
        check_lexer(
            "note:  # why\n  text\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Whitespace, "  "),
                (Comment, "# why"),
                (Newline, "\n"),
                (MarkdownBlock, "  text\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_colon_followed_by_a_token_opens_no_body() {
        check_lexer(
            "note: x\n  text\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Whitespace, " "),
                (Word, "x"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "text"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_line_not_indented_more_than_the_opening_line_is_not_a_body() {
        check_lexer(
            "note:\nnext\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (Word, "next"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_comment_line_not_indented_more_ends_the_wait_for_a_body() {
        check_lexer(
            "note:\n# aside\n  text\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (Comment, "# aside"),
                (BlankLine, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "text"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_tab_does_not_indent_a_body() {
        check_lexer(
            "note:\n\ttext\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (Error, "\t"),
                (Word, "text"),
                (Newline, "\n"),
            ],
            &[error(IllegalTab, 6, 1)],
        );
    }

    #[test]
    fn a_colon_at_the_end_of_input_has_no_body() {
        check_lexer("note:", &[(Word, "note"), (Colon, ":"), (Newline, "")], &[]);
    }

    #[test]
    fn blank_lines_before_a_body_are_blank_lines() {
        check_lexer(
            "note:\n\n   \n  text\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (BlankLine, "\n"),
                (Whitespace, "   "),
                (BlankLine, "\n"),
                (MarkdownBlock, "  text\n"),
            ],
            &[],
        );
    }

    #[test]
    fn each_field_gets_its_own_body() {
        check_lexer(
            "a:\n  one\nb:\n  two\n",
            &[
                (Word, "a"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  one\n"),
                (Word, "b"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  two\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_must_be_deeper_than_its_opening_line() {
        check_lexer(
            "a\n  note:\n  next\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Word, "next"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_leaves_the_enclosing_blocks_unchanged() {
        check_lexer(
            "a\n  note:\n    text\n  b\nc\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "    text\n"),
                (Whitespace, "  "),
                (Word, "b"),
                (Newline, "\n"),
                (Dedent, ""),
                (Word, "c"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_may_close_blocks_after_it() {
        check_lexer(
            "a\n  note:\n    text\nc\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "    text\n"),
                (Dedent, ""),
                (Word, "c"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_keeps_its_carriage_returns() {
        check_lexer(
            "note:\r\n  one\r\n  two\rnext\r\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\r\n"),
                (MarkdownBlock, "  one\r\n  two\r"),
                (Word, "next"),
                (Newline, "\r\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_blank_line_inside_a_body_is_part_of_it() {
        check_lexer(
            "note:\n  one\n\n  two\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  one\n\n  two\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_may_hold_several_blank_lines_of_any_width() {
        check_lexer(
            "note:\n  one\n\n \n      \n  two\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  one\n\n \n      \n  two\n"),
            ],
            &[],
        );
    }

    #[test]
    fn blank_lines_after_a_body_are_blank_lines() {
        check_lexer(
            "note:\n  text\n\n  \nnext\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  text\n"),
                (BlankLine, "\n"),
                (Whitespace, "  "),
                (BlankLine, "\n"),
                (Word, "next"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn blank_lines_at_the_end_of_input_are_not_part_of_a_body() {
        check_lexer(
            "note:\n  text\n\n   ",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  text\n"),
                (BlankLine, "\n"),
                (Whitespace, "   "),
            ],
            &[],
        );
    }

    #[test]
    fn a_body_ends_before_the_blank_lines_ahead_of_a_shallower_line() {
        check_lexer(
            "a\n  note:\n    one\n\n    two\n\n  b\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "    one\n\n    two\n"),
                (BlankLine, "\n"),
                (Whitespace, "  "),
                (Word, "b"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[],
        );
    }
}
