use super::{BYTE_ORDER_MARK, Cursor};
use crate::{LexicalErrorKind, TokenKind};

impl Cursor<'_> {
    pub(super) fn starts_ignored_token(&self, c: char) -> bool {
        matches!(c, ' ' | '#' | '\n' | '\r') || (c == BYTE_ORDER_MARK && self.pos == 0)
    }

    pub(super) fn begin_line(&mut self, start: usize) {
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
