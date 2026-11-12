use crate::parser::Parser;
use crate::{Keyword, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn block(&mut self, mut entry: impl FnMut(&mut Self, &mut Vec<Keyword>) -> bool) {
        self.start_node(SyntaxKind::Block);
        self.bump();
        let mut seen = Vec::new();
        while !self.at(TokenKind::Dedent) && self.current().is_some() {
            if !entry(self, &mut seen) {
                break;
            }
        }
        self.eat(TokenKind::Dedent);
        self.finish_node();
    }
}
