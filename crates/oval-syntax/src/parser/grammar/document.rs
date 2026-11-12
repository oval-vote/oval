use crate::parser::Parser;
use crate::{Keyword, SyntaxKind};

impl Parser<'_> {
    pub(in crate::parser) fn document(&mut self) {
        self.start_node_here(SyntaxKind::Document);
        if self.at_keyword(Keyword::Oval) {
            self.version_marker();
        }
        let mut seen = Vec::new();
        while self.guide_field(&mut seen) {}
        while self.declaration() {}
        while self.entry_head().keyword == Some(Keyword::Ballot) {
            self.ballot();
        }
        while self.current().is_some() {
            self.bump();
        }
        self.eat_skipped();
        self.finish_node();
    }
}
