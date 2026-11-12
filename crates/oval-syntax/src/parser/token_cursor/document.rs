use super::TokenCursor;
use crate::{Keyword, SyntaxKind};

impl TokenCursor<'_> {
    pub(crate) fn document(&mut self) {
        self.start_node_here(SyntaxKind::Document);
        if self.at_keyword(Keyword::Oval) {
            self.version_marker();
        }
        while self.current().is_some() {
            self.bump();
        }
        self.skip_ignored();
        self.finish_node();
    }
}
