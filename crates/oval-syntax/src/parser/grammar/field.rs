use crate::parser::Parser;
use crate::{SyntaxErrorKind, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn value_field(&mut self, node: SyntaxKind, value: TokenKind) {
        self.start_node(node);
        self.bump();
        self.expect(value, SyntaxErrorKind::MissingValue);
        self.eat(TokenKind::Newline);
        self.finish_node();
    }
}
