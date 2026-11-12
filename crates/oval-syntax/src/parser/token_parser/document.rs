use super::TokenParser;
use crate::SyntaxKind;

impl TokenParser<'_> {
    pub(crate) fn document(&mut self) {
        self.start_node(SyntaxKind::Document);
        if self.at_keyword("oval") {
            self.version_marker();
        }
        while self.current().is_some() {
            self.bump();
        }
        self.finish_node();
    }
}
