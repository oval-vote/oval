mod document;
mod version_marker;

use rowan::{GreenNode, GreenNodeBuilder, Language};

use crate::{Keyword, OvalLanguage, SyntaxKind, Token, TokenKind};

pub(super) struct TokenParser<'t> {
    text: &'t str,
    tokens: Vec<Token>,
    pos: usize,
    offset: usize,
    builder: GreenNodeBuilder<'static>,
}

impl<'t> TokenParser<'t> {
    pub(super) fn new(text: &'t str, tokens: Vec<Token>) -> Self {
        TokenParser {
            text,
            tokens,
            pos: 0,
            offset: 0,
            builder: GreenNodeBuilder::new(),
        }
    }

    pub(super) fn current(&mut self) -> Option<TokenKind> {
        while let Some(token) = self.tokens.get(self.pos).copied()
            && token.is_ignored()
        {
            self.add_token(token);
        }
        self.tokens.get(self.pos).map(|token| token.kind)
    }

    fn at(&mut self, kind: TokenKind) -> bool {
        self.current() == Some(kind)
    }

    fn at_keyword(&mut self, keyword: Keyword) -> bool {
        self.at(TokenKind::Word) && {
            let len = self.tokens[self.pos].len as usize;
            &self.text[self.offset..self.offset + len] == keyword.text()
        }
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        let at = self.at(kind);
        if at {
            self.bump();
        }
        at
    }

    pub(super) fn bump(&mut self) {
        assert!(
            self.current().is_some(),
            "bump is only called before the end of input"
        );
        self.add_token(self.tokens[self.pos]);
    }

    pub(super) fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(OvalLanguage::kind_to_raw(kind));
    }

    pub(super) fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    pub(super) fn finish(self) -> GreenNode {
        self.builder.finish()
    }

    fn add_token(&mut self, token: Token) {
        let end = self.offset + token.len as usize;
        self.builder.token(
            OvalLanguage::kind_to_raw(token.kind.into()),
            &self.text[self.offset..end],
        );
        self.pos += 1;
        self.offset = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyntaxNode, lex};

    fn parser(text: &str) -> TokenParser<'_> {
        let mut parser = TokenParser::new(text, lex(text).unwrap().tokens);
        parser.start_node(SyntaxKind::Document);
        parser
    }

    fn tree(mut parser: TokenParser<'_>) -> SyntaxNode {
        parser.finish_node();
        SyntaxNode::new_root(parser.finish())
    }

    #[test]
    fn current_skips_ignored_tokens_but_keeps_them_in_the_tree() {
        let mut parser = parser("  # aside\n\nword\n");
        assert_eq!(parser.current(), Some(TokenKind::Word));
        assert_eq!(tree(parser).to_string(), "  # aside\n\n");
    }

    #[test]
    fn current_does_not_skip_layout_tokens() {
        let mut parser = parser("a\n  b\n");
        parser.bump();
        assert_eq!(parser.current(), Some(TokenKind::Newline));
        parser.bump();
        assert_eq!(parser.current(), Some(TokenKind::Indent));
    }

    #[test]
    fn current_is_none_at_the_end_of_input() {
        let mut parser = parser("word");
        parser.bump();
        parser.bump();
        assert_eq!(parser.current(), None);
    }

    #[test]
    fn bump_adds_the_token_after_any_ignored_tokens() {
        let mut parser = parser("# aside\nword");
        parser.bump();
        assert_eq!(tree(parser).to_string(), "# aside\nword");
    }

    #[test]
    #[should_panic(expected = "bump is only called before the end of input")]
    fn bump_at_the_end_of_input_panics() {
        let mut parser = parser("");
        parser.bump();
    }
}
