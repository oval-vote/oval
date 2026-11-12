use crate::parser::Parser;
use crate::{SyntaxErrorKind, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn value_field(&mut self, node: SyntaxKind, value: TokenKind) {
        self.start_node(node);
        self.bump();
        self.expect(value, SyntaxErrorKind::MissingValue);
        self.end_of_line();
        self.finish_node();
    }
}
