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

    pub(super) fn current(&self) -> Option<TokenKind> {
        self.peek().map(|(token, _)| token.kind)
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current() == Some(kind)
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        self.peek().is_some_and(|(token, offset)| {
            token.kind == TokenKind::Word
                && &self.text[offset..offset + token.len as usize] == keyword.text()
        })
    }

    fn peek(&self) -> Option<(Token, usize)> {
        let mut offset = self.offset;
        for &token in &self.tokens[self.pos..] {
            if !token.is_ignored() {
                return Some((token, offset));
            }
            offset += token.len as usize;
        }
        None
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        let at = self.at(kind);
        if at {
            self.bump();
        }
        at
    }

    pub(super) fn bump(&mut self) {
        self.skip_ignored();
        let token = *self
            .tokens
            .get(self.pos)
            .expect("bump is only called before the end of input");
        self.add_token(token);
    }

    pub(super) fn start_node(&mut self, kind: SyntaxKind) {
        self.skip_ignored();
        self.start_node_here(kind);
    }

    fn start_node_here(&mut self, kind: SyntaxKind) {
        self.builder.start_node(OvalLanguage::kind_to_raw(kind));
    }

    pub(super) fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    pub(super) fn finish(self) -> GreenNode {
        self.builder.finish()
    }

    fn skip_ignored(&mut self) {
        while let Some(token) = self.tokens.get(self.pos).copied()
            && token.is_ignored()
        {
            self.add_token(token);
        }
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
    use expect_test::expect;

    fn parser(text: &str) -> TokenParser<'_> {
        let mut parser = TokenParser::new(text, lex(text).unwrap().tokens);
        parser.start_node_here(SyntaxKind::Document);
        parser
    }

    fn tree(mut parser: TokenParser<'_>) -> SyntaxNode {
        parser.finish_node();
        SyntaxNode::new_root(parser.finish())
    }

    #[test]
    fn current_looks_past_ignored_tokens_without_taking_them() {
        let parser = parser("  # aside\n\nword\n");
        assert_eq!(parser.current(), Some(TokenKind::Word));
        assert_eq!(tree(parser).to_string(), "");
    }

    #[test]
    fn at_keyword_looks_past_ignored_tokens() {
        let parser = parser("# aside\noval 1\n");
        assert!(parser.at_keyword(Keyword::Oval));
        assert!(!parser.at_keyword(Keyword::Guide));
    }

    #[test]
    fn start_node_puts_ignored_tokens_before_the_node() {
        let mut parser = parser("# aside\noval");
        parser.start_node(SyntaxKind::VersionMarker);
        parser.bump();
        parser.finish_node();
        expect![[r##"
            Document@0..12
              Comment@0..7 "# aside"
              BlankLine@7..8 "\n"
              VersionMarker@8..12
                Word@8..12 "oval"
        "##]]
        .assert_eq(&format!("{:#?}", tree(parser)));
    }

    #[test]
    fn skip_ignored_takes_ignored_tokens_up_to_the_next_real_one() {
        let mut parser = parser("# aside\nword");
        parser.skip_ignored();
        assert_eq!(parser.current(), Some(TokenKind::Word));
        assert_eq!(tree(parser).to_string(), "# aside\n");
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
