use crate::parser::Parser;
use crate::{Keyword, SyntaxKind};

impl Parser<'_> {
    pub(in crate::parser) fn document(&mut self) {
        self.start_node_here(SyntaxKind::Document);
        if self.at_keyword(Keyword::Oval) {
            self.version_marker();
        }
        while self.current().is_some() {
            self.bump();
        }
        self.eat_skipped();
        self.finish_node();
    }
}
