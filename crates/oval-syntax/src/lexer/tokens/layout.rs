use crate::lexer::{BYTE_ORDER_MARK, Lexer};
use crate::{LexicalErrorKind, TokenKind};

impl Lexer<'_> {
    pub(in crate::lexer) fn starts_ignored_token(&self, c: char) -> bool {
        matches!(c, ' ' | '#' | '\n' | '\r') || (c == BYTE_ORDER_MARK && self.pos == 0)
    }

    pub(in crate::lexer) fn line_terminator(&mut self, c: char, start: usize) {
        self.bump(c);
        if c == '\r' && self.peek() == Some('\n') {
            self.bump('\n');
        }
        let kind = if self.line_has_tokens {
            self.body_opener = self.line_ends_with_colon.then_some(self.line_indentation);
            TokenKind::Newline
        } else {
            TokenKind::BlankLine
        };
        self.push(kind, start);
        self.line_has_tokens = false;
        self.line_start = self.pos;
    }

    pub(in crate::lexer) fn begin_line(&mut self, start: usize) {
        let indentation = start - self.line_start;
        self.line_indentation = indentation;
        if indentation > self.current_indentation() {
            self.indent_stack.push(indentation);
            self.push(TokenKind::Indent, start);
        } else {
            while indentation < self.current_indentation() {
                self.indent_stack.pop();
                self.push(TokenKind::Dedent, start);
            }
            if indentation != self.current_indentation() {
                self.report(LexicalErrorKind::InconsistentDedent, self.line_start, start);
            }
        }
    }

    pub(in crate::lexer) fn end_input(&mut self) {
        if self.line_has_tokens {
            self.push(TokenKind::Newline, self.pos);
        }
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.push(TokenKind::Dedent, self.pos);
        }
    }

    fn current_indentation(&self) -> usize {
        *self
            .indent_stack
            .last()
            .expect("the indentation stack always holds level 0")
    }
}

#[cfg(test)]
mod tests {
    use crate::LexicalErrorKind::{IllegalTab, InconsistentDedent, UnexpectedCharacter};
    use crate::TokenKind::{
        BlankLine, ByteOrderMark, Comment, Dedent, Error, Indent, Newline, Whitespace, Word,
    };
    use crate::lexer::tests::{check_lexer, error};

    #[test]
    fn a_line_feed_after_tokens_is_a_newline() {
        check_lexer("a\n", &[(Word, "a"), (Newline, "\n")], &[]);
    }

    #[test]
    fn a_line_feed_on_an_empty_line_is_a_blank_line() {
        check_lexer("\n", &[(BlankLine, "\n")], &[]);
    }

    #[test]
    fn a_line_of_whitespace_and_a_comment_is_a_blank_line() {
        check_lexer(
            "  # x\n",
            &[(Whitespace, "  "), (Comment, "# x"), (BlankLine, "\n")],
            &[],
        );
    }

    #[test]
    fn a_line_with_only_a_byte_order_mark_is_a_blank_line() {
        check_lexer(
            "\u{FEFF}\n",
            &[(ByteOrderMark, "\u{FEFF}"), (BlankLine, "\n")],
            &[],
        );
    }

    #[test]
    fn a_comment_after_tokens_keeps_the_newline() {
        check_lexer(
            "a # x\n",
            &[
                (Word, "a"),
                (Whitespace, " "),
                (Comment, "# x"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_line_with_an_error_ends_in_a_newline() {
        check_lexer(
            "-\n",
            &[(Error, "-"), (Newline, "\n")],
            &[error(UnexpectedCharacter, 0, 1)],
        );
    }

    #[test]
    fn each_line_is_judged_on_its_own_tokens() {
        check_lexer(
            "a\n\nb\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (BlankLine, "\n"),
                (Word, "b"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn every_line_terminator_ends_a_line() {
        for terminator in ["\n", "\r\n", "\r"] {
            check_lexer(
                &format!("a{terminator}{terminator}"),
                &[(Word, "a"), (Newline, terminator), (BlankLine, terminator)],
                &[],
            );
        }
    }

    #[test]
    fn a_carriage_return_and_line_feed_are_one_terminator() {
        check_lexer("\r\n", &[(BlankLine, "\r\n")], &[]);
    }

    #[test]
    fn a_line_feed_before_a_carriage_return_is_two_terminators() {
        check_lexer("\n\r", &[(BlankLine, "\n"), (BlankLine, "\r")], &[]);
    }

    #[test]
    fn a_carriage_return_before_a_crlf_is_two_terminators() {
        check_lexer(
            "a\r\r\n",
            &[(Word, "a"), (Newline, "\r"), (BlankLine, "\r\n")],
            &[],
        );
    }

    #[test]
    fn terminators_may_be_mixed_in_one_document() {
        check_lexer(
            "a\r\nb\rc\n",
            &[
                (Word, "a"),
                (Newline, "\r\n"),
                (Word, "b"),
                (Newline, "\r"),
                (Word, "c"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn a_comment_line_ending_in_crlf_is_a_blank_line() {
        check_lexer("# x\r\n", &[(Comment, "# x"), (BlankLine, "\r\n")], &[]);
    }

    #[test]
    fn a_last_line_without_a_terminator_ends_in_an_empty_newline() {
        check_lexer("a", &[(Word, "a"), (Newline, "")], &[]);
    }

    #[test]
    fn a_last_line_with_a_terminator_gets_no_extra_newline() {
        check_lexer("a\n", &[(Word, "a"), (Newline, "\n")], &[]);
    }

    #[test]
    fn a_last_line_with_only_ignored_tokens_gets_no_newline() {
        check_lexer("  ", &[(Whitespace, "  ")], &[]);
        check_lexer("# c", &[(Comment, "# c")], &[]);
        check_lexer("\u{FEFF}", &[(ByteOrderMark, "\u{FEFF}")], &[]);
        check_lexer(
            "a\n  # c",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Comment, "# c"),
            ],
            &[],
        );
    }

    #[test]
    fn a_last_line_with_only_an_error_ends_in_an_empty_newline() {
        check_lexer(
            "-",
            &[(Error, "-"), (Newline, "")],
            &[error(UnexpectedCharacter, 0, 1)],
        );
    }

    #[test]
    fn a_blank_last_line_gets_no_newline() {
        check_lexer(
            "a\n\n",
            &[(Word, "a"), (Newline, "\n"), (BlankLine, "\n")],
            &[],
        );
    }

    #[test]
    fn an_indented_line_opens_a_block_that_closes_at_the_end_of_input() {
        check_lexer(
            "a\n  b\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_line_back_at_an_outer_level_closes_the_block() {
        check_lexer(
            "a\n  b\nc\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
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
    fn a_line_at_the_same_level_adds_no_layout_tokens() {
        check_lexer(
            "  a\n  b\n",
            &[
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Word, "b"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn one_line_may_close_several_blocks() {
        check_lexer(
            "a\n  b\n    c\nd\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (Whitespace, "    "),
                (Indent, ""),
                (Word, "c"),
                (Newline, "\n"),
                (Dedent, ""),
                (Dedent, ""),
                (Word, "d"),
                (Newline, "\n"),
            ],
            &[],
        );
    }

    #[test]
    fn the_end_of_input_closes_every_open_block() {
        check_lexer(
            "a\n  b\n    c",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (Whitespace, "    "),
                (Indent, ""),
                (Word, "c"),
                (Newline, ""),
                (Dedent, ""),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn indentation_width_is_free() {
        check_lexer(
            "a\n   b\n       c\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "   "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (Whitespace, "       "),
                (Indent, ""),
                (Word, "c"),
                (Newline, "\n"),
                (Dedent, ""),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn blank_and_comment_lines_do_not_affect_layout() {
        check_lexer(
            "a\n  b\n\n# outer\n      # deeper\n  c\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (BlankLine, "\n"),
                (Comment, "# outer"),
                (BlankLine, "\n"),
                (Whitespace, "      "),
                (Comment, "# deeper"),
                (BlankLine, "\n"),
                (Whitespace, "  "),
                (Word, "c"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn an_indented_first_line_opens_a_block() {
        check_lexer(
            "  a",
            &[
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "a"),
                (Newline, ""),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_leading_tab_is_at_indentation_zero() {
        // A tab ends a line's indentation, so this line is at indentation 0 and opens no block.
        check_lexer(
            "\ta",
            &[(Error, "\t"), (Word, "a"), (Newline, "")],
            &[error(IllegalTab, 0, 1)],
        );
    }

    #[test]
    fn spaces_before_a_tab_are_indentation() {
        check_lexer(
            "  \ta",
            &[
                (Whitespace, "  "),
                (Indent, ""),
                (Error, "\t"),
                (Word, "a"),
                (Newline, ""),
                (Dedent, ""),
            ],
            &[error(IllegalTab, 2, 1)],
        );
    }

    #[test]
    fn indentation_is_counted_after_a_byte_order_mark() {
        check_lexer(
            "\u{FEFF}  a",
            &[
                (ByteOrderMark, "\u{FEFF}"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "a"),
                (Newline, ""),
                (Dedent, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_byte_order_mark_alone_is_not_indentation() {
        check_lexer(
            "\u{FEFF}a",
            &[(ByteOrderMark, "\u{FEFF}"), (Word, "a"), (Newline, "")],
            &[],
        );
    }

    #[test]
    fn layout_works_with_carriage_returns() {
        check_lexer(
            "a\r\n  b\rc",
            &[
                (Word, "a"),
                (Newline, "\r\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\r"),
                (Dedent, ""),
                (Word, "c"),
                (Newline, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_line_between_two_levels_is_an_inconsistent_dedent() {
        // The specification's example for the inconsistent-dedent error: three spaces match no
        // enclosing level, so the line closes one block and is treated as being at the section's level.
        check_lexer(
            "a\n  b\n    c\n   d\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (Whitespace, "    "),
                (Indent, ""),
                (Word, "c"),
                (Newline, "\n"),
                (Whitespace, "   "),
                (Dedent, ""),
                (Word, "d"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[error(InconsistentDedent, 12, 3)],
        );
    }

    #[test]
    fn a_line_after_an_inconsistent_dedent_continues_at_the_recovered_level() {
        check_lexer(
            "a\n  b\n    c\n   d\n  e\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (Whitespace, "    "),
                (Indent, ""),
                (Word, "c"),
                (Newline, "\n"),
                (Whitespace, "   "),
                (Dedent, ""),
                (Word, "d"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Word, "e"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
            &[error(InconsistentDedent, 12, 3)],
        );
    }

    #[test]
    fn an_inconsistent_dedent_may_return_to_the_outermost_level() {
        // One space is between levels 0 and 2, so the line closes the block and is treated as being
        // at level 0.
        check_lexer(
            "a\n  b\n c\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (Whitespace, " "),
                (Dedent, ""),
                (Word, "c"),
                (Newline, "\n"),
            ],
            &[error(InconsistentDedent, 6, 1)],
        );
    }

    #[test]
    fn a_deeper_line_after_an_inconsistent_dedent_opens_a_block() {
        // After recovering to level 2, a line at 3 is deeper and opens a new block.
        check_lexer(
            "a\n  b\n    c\n   d\n   e\n",
            &[
                (Word, "a"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "b"),
                (Newline, "\n"),
                (Whitespace, "    "),
                (Indent, ""),
                (Word, "c"),
                (Newline, "\n"),
                (Whitespace, "   "),
                (Dedent, ""),
                (Word, "d"),
                (Newline, "\n"),
                (Whitespace, "   "),
                (Indent, ""),
                (Word, "e"),
                (Newline, "\n"),
                (Dedent, ""),
                (Dedent, ""),
            ],
            &[error(InconsistentDedent, 12, 3)],
        );
    }
}
