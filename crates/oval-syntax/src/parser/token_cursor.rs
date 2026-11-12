mod document;
mod version_marker;

use rowan::{GreenNode, GreenNodeBuilder, Language};

use crate::lexer::to_u32;
use crate::{
    Expected, Keyword, OvalLanguage, SyntaxError, SyntaxErrorKind, SyntaxKind, Token, TokenKind,
};

pub(super) struct TokenCursor<'t> {
    text: &'t str,
    tokens: Vec<Token>,
    pos: usize,
    offset: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
}

impl<'t> TokenCursor<'t> {
    pub(super) fn new(text: &'t str, tokens: Vec<Token>) -> Self {
        TokenCursor {
            text,
            tokens,
            pos: 0,
            offset: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
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

    fn at_line_end(&self) -> bool {
        matches!(
            self.current(),
            None | Some(TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent)
        )
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        let at = self.at(kind);
        if at {
            self.bump();
        }
        at
    }

    fn expect(&mut self, kind: TokenKind, missing: SyntaxErrorKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        if self.at_line_end() {
            let offset = self.peek().map_or(self.text.len(), |(_, offset)| offset);
            self.errors.push(SyntaxError {
                kind: missing,
                offset: to_u32(offset),
                len: 0,
                expected: vec![Expected::Token(kind)],
            });
        }
        false
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

    pub(super) fn finish(self) -> (GreenNode, Vec<SyntaxError>) {
        (self.builder.finish(), self.errors)
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

    fn cursor(text: &str) -> TokenCursor<'_> {
        let mut cursor = TokenCursor::new(text, lex(text).unwrap().tokens);
        cursor.start_node_here(SyntaxKind::Document);
        cursor
    }

    fn tree(mut cursor: TokenCursor<'_>) -> SyntaxNode {
        cursor.finish_node();
        SyntaxNode::new_root(cursor.finish().0)
    }

    fn missing_integer(offset: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::MissingValue,
            offset,
            len: 0,
            expected: vec![Expected::Token(TokenKind::Integer)],
        }
    }

    #[test]
    fn current_looks_past_ignored_tokens_without_taking_them() {
        let cursor = cursor("  # aside\n\nword\n");
        assert_eq!(cursor.current(), Some(TokenKind::Word));
        assert_eq!(tree(cursor).to_string(), "");
    }

    #[test]
    fn at_keyword_looks_past_ignored_tokens() {
        let cursor = cursor("# aside\noval 1\n");
        assert!(cursor.at_keyword(Keyword::Oval));
        assert!(!cursor.at_keyword(Keyword::Guide));
    }

    #[test]
    fn start_node_puts_ignored_tokens_before_the_node() {
        let mut cursor = cursor("# aside\noval");
        cursor.start_node(SyntaxKind::VersionMarker);
        cursor.bump();
        cursor.finish_node();
        expect![[r##"
            Document@0..12
              Comment@0..7 "# aside"
              BlankLine@7..8 "\n"
              VersionMarker@8..12
                Word@8..12 "oval"
        "##]]
        .assert_eq(&format!("{:#?}", tree(cursor)));
    }

    #[test]
    fn skip_ignored_takes_ignored_tokens_up_to_the_next_real_one() {
        let mut cursor = cursor("# aside\nword");
        cursor.skip_ignored();
        assert_eq!(cursor.current(), Some(TokenKind::Word));
        assert_eq!(tree(cursor).to_string(), "# aside\n");
    }

    #[test]
    fn current_does_not_skip_layout_tokens() {
        let mut cursor = cursor("a\n  b\n");
        cursor.bump();
        assert_eq!(cursor.current(), Some(TokenKind::Newline));
        cursor.bump();
        assert_eq!(cursor.current(), Some(TokenKind::Indent));
    }

    #[test]
    fn current_is_none_at_the_end_of_input() {
        let mut cursor = cursor("word");
        cursor.bump();
        cursor.bump();
        assert_eq!(cursor.current(), None);
    }

    #[test]
    fn bump_adds_the_token_after_any_ignored_tokens() {
        let mut cursor = cursor("# aside\nword");
        cursor.bump();
        assert_eq!(tree(cursor).to_string(), "# aside\nword");
    }

    #[test]
    #[should_panic(expected = "bump is only called before the end of input")]
    fn bump_at_the_end_of_input_panics() {
        let mut cursor = cursor("");
        cursor.bump();
    }

    #[test]
    fn expect_takes_the_token_when_it_is_there() {
        let mut cursor = cursor("1\n");
        assert!(cursor.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(cursor.current(), Some(TokenKind::Newline));
        assert_eq!(cursor.errors, []);
    }

    #[test]
    fn expect_reports_a_missing_token_at_the_end_of_the_line() {
        let mut cursor = cursor("oval  \n");
        cursor.bump();
        assert!(!cursor.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(cursor.errors, [missing_integer(6)]);
    }

    #[test]
    fn expect_reports_a_missing_token_at_the_end_of_input() {
        let mut cursor = cursor("");
        assert!(!cursor.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(cursor.errors, [missing_integer(0)]);
    }

    #[test]
    fn expect_reports_nothing_when_another_token_is_there() {
        let mut cursor = cursor("x\n");
        assert!(!cursor.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(cursor.current(), Some(TokenKind::Word));
        assert_eq!(cursor.errors, []);
    }
}
