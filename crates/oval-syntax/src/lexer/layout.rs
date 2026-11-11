use super::Lexer;
use crate::TokenKind;

impl Lexer<'_> {
    // Ends the last line. A line with tokens but no line terminator still ends in a `Newline`, so
    // every line with tokens ends the same way.
    pub(super) fn end_input(&mut self) {
        if self.line_has_tokens {
            self.push(TokenKind::Newline, self.pos);
        }
    }
}
