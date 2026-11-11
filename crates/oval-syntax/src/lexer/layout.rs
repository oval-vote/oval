use super::{BYTE_ORDER_MARK, Lexer};
use crate::{LexicalErrorKind, TokenKind};

impl Lexer<'_> {
    // Whether `c`, at the current position, begins a token the parser ignores. Layout is decided by
    // a line's first token that is not ignored.
    pub(super) fn starts_ignored_token(&self, c: char) -> bool {
        matches!(c, ' ' | '#' | '\n' | '\r') || (c == BYTE_ORDER_MARK && self.pos == 0)
    }

    // Emits the layout tokens before the first token of a line that has one, by comparing the
    // line's indentation with the enclosing blocks.
    pub(super) fn begin_line(&mut self, start: usize) {
        // Only spaces can come before a line's first token, since anything else would be that token.
        let indentation = start - self.line_start;
        if indentation > self.current_indentation() {
            self.indent_stack.push(indentation);
            self.push(TokenKind::Indent, start);
        } else {
            while indentation < self.current_indentation() {
                self.indent_stack.pop();
                self.push(TokenKind::Dedent, start);
            }
            // A line between two levels is reported, and then treated as being at the level it
            // stopped at. Its leading spaces stay a `Whitespace` token.
            if indentation != self.current_indentation() {
                self.report(LexicalErrorKind::InconsistentDedent, self.line_start, start);
            }
        }
    }

    // Ends the last line. A line with tokens but no line terminator still ends in a `Newline`, so
    // every line with tokens ends the same way, and every block still open is closed.
    pub(super) fn end_input(&mut self) {
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
