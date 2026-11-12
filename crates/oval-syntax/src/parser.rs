mod grammar;
mod parse_result;
mod syntax_error;

pub use parse_result::ParseResult;
pub use syntax_error::{Expected, SyntaxError, SyntaxErrorKind};

use rowan::{GreenNode, GreenNodeBuilder, Language};

use crate::lexer::to_u32;
use crate::{InputRejection, Keyword, LexResult, OvalLanguage, SyntaxKind, Token, TokenKind, lex};

/// Parses an Oval document into a syntax tree.
///
/// The tree holds every token of the document, so its text is always exactly `text`. Parsing never
/// stops at an error. The tree is complete even when the document has lexical or syntax errors,
/// which the [`ParseResult`] reports alongside it.
///
/// Only the version marker is parsed so far. Every other token is a direct child of the
/// [`Document`](crate::SyntaxKind::Document) node.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `text` cannot be lexed at all. See [`lex`].
///
/// # Examples
///
/// ```
/// use oval_syntax::{SyntaxKind, parse};
///
/// let text = "oval 1\n";
/// let result = parse(text)?;
/// let tree = result.syntax();
/// assert_eq!(tree.kind(), SyntaxKind::Document);
/// assert_eq!(tree.to_string(), text);
/// assert!(result.lexical_errors.is_empty());
/// assert!(result.syntax_errors.is_empty());
/// # Ok::<(), oval_syntax::InputRejection>(())
/// ```
pub fn parse(text: &str) -> Result<ParseResult, InputRejection> {
    let LexResult { tokens, errors } = lex(text)?;
    let mut parser = Parser::new(text, tokens);
    parser.document();
    let (green, syntax_errors) = parser.finish();
    Ok(ParseResult {
        green,
        lexical_errors: errors,
        syntax_errors,
    })
}

struct Parser<'t> {
    text: &'t str,
    tokens: Vec<Token>,
    pos: usize,
    offset: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
    line_start: usize,
    line_is_empty: bool,
    line_end: usize,
    line_was_skipped: bool,
}

impl<'t> Parser<'t> {
    fn new(text: &'t str, tokens: Vec<Token>) -> Self {
        Parser {
            text,
            tokens,
            pos: 0,
            offset: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            line_start: 0,
            line_is_empty: true,
            line_end: 0,
            line_was_skipped: false,
        }
    }

    fn current(&self) -> Option<TokenKind> {
        self.peek().map(|(token, _)| token.kind)
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current() == Some(kind)
    }

    fn current_keyword(&self) -> Option<Keyword> {
        let (token, offset) = self.peek()?;
        if token.kind != TokenKind::Word {
            return None;
        }
        Keyword::from_text(&self.text[offset..offset + token.len as usize])
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        self.current_keyword() == Some(keyword)
    }

    fn peek(&self) -> Option<(Token, usize)> {
        let mut offset = self.offset;
        for &token in &self.tokens[self.pos..] {
            if !is_skipped(token, self.line_is_empty) {
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
        if !self.at_line_end() {
            self.skip_line(vec![Expected::Token(kind)]);
        } else if !self.line_has_error() {
            self.report(
                missing,
                self.line_end,
                self.line_end,
                vec![Expected::Token(kind)],
            );
        }
        false
    }

    fn end_of_line(&mut self) {
        if self.finish_line() {
            self.discard_body();
        }
    }

    fn finish_line(&mut self) -> bool {
        if !self.at_line_end() {
            self.skip_line(vec![Expected::Token(TokenKind::Newline)]);
        }
        let was_skipped = self.line_was_skipped;
        self.eat(TokenKind::Newline);
        was_skipped
    }

    fn discard_body(&mut self) {
        if self.at(TokenKind::MarkdownBlock) {
            self.start_node(SyntaxKind::ErrorNode);
            self.bump();
            self.finish_node();
        }
    }

    fn skip_line(&mut self, expected: Vec<Expected>) {
        self.start_node(SyntaxKind::ErrorNode);
        let start = self.offset;
        while !self.at_line_end() {
            self.bump();
        }
        self.finish_node();
        self.line_was_skipped = true;
        self.report(
            SyntaxErrorKind::UnexpectedToken,
            start,
            self.line_end,
            expected,
        );
    }

    fn report(&mut self, kind: SyntaxErrorKind, start: usize, end: usize, expected: Vec<Expected>) {
        self.errors.push(SyntaxError {
            kind,
            offset: to_u32(start),
            len: to_u32(end - start),
            expected,
        });
    }

    fn line_has_error(&self) -> bool {
        self.tokens[self.line_start..]
            .iter()
            .take_while(|token| !ends_line(**token))
            .any(|token| token.is_error())
    }

    fn bump(&mut self) {
        self.eat_skipped();
        let token = *self
            .tokens
            .get(self.pos)
            .expect("bump is only called before the end of input");
        self.add_token(token);
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        self.eat_skipped();
        self.start_node_here(kind);
    }

    fn start_node_here(&mut self, kind: SyntaxKind) {
        self.builder.start_node(OvalLanguage::kind_to_raw(kind));
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    fn finish(self) -> (GreenNode, Vec<SyntaxError>) {
        (self.builder.finish(), self.errors)
    }

    fn eat_skipped(&mut self) {
        while let Some(token) = self.tokens.get(self.pos).copied()
            && is_skipped(token, self.line_is_empty)
        {
            self.add_token(token);
        }
    }

    fn add_token(&mut self, token: Token) {
        let skipped = is_skipped(token, self.line_is_empty);
        let end = self.offset + token.len as usize;
        self.builder.token(
            OvalLanguage::kind_to_raw(token.kind.into()),
            &self.text[self.offset..end],
        );
        self.pos += 1;
        self.offset = end;
        if !skipped && token.kind != TokenKind::Newline {
            self.line_end = end;
        }
        if ends_line(token) {
            self.line_start = self.pos;
            self.line_is_empty = true;
            self.line_was_skipped = false;
        } else if !token.is_ignored() && !token.is_error() {
            self.line_is_empty = false;
        }
    }
}

fn is_skipped(token: Token, line_is_empty: bool) -> bool {
    token.is_ignored() || token.is_error() || (token.kind == TokenKind::Newline && line_is_empty)
}

fn ends_line(token: Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Newline | TokenKind::BlankLine | TokenKind::MarkdownBlock
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LexicalError, LexicalErrorKind, SyntaxNode};
    use expect_test::{Expect, expect};

    pub(super) fn check(text: &str, expected: &Expect) {
        let tree = parse(text).expect("the text is accepted").syntax();
        expected.assert_eq(&format!("{tree:#?}"));
    }

    fn check_round_trip(text: &str) {
        let tree = parse(text).expect("the text is accepted").syntax();
        assert_eq!(tree.to_string(), text);
    }

    #[test]
    fn the_root_is_a_document() {
        let tree = parse("date 2026-11-03\n").unwrap().syntax();
        assert_eq!(tree.kind(), SyntaxKind::Document);
    }

    #[test]
    fn a_clean_document_has_no_errors() {
        let result = parse("oval 1\nguide \"G\"\n").unwrap();
        assert_eq!(result.lexical_errors, []);
        assert_eq!(result.syntax_errors, []);
    }

    #[test]
    fn lexical_errors_are_reported_alongside_the_tree() {
        let text = "subtitle\t\"G\"\n";
        let result = parse(text).unwrap();
        assert_eq!(
            result.lexical_errors,
            [LexicalError {
                kind: LexicalErrorKind::IllegalTab,
                offset: 8,
                len: 1,
            }]
        );
        assert_eq!(result.syntax_errors, []);
        assert_eq!(result.syntax().to_string(), text);
    }

    #[test]
    fn every_token_is_inside_the_document() {
        check(
            "guide \"G\"\n  note:\n    Text.\n",
            &expect![[r#"
                Document@0..28
                  StringField@0..10
                    Word@0..5 "guide"
                    Whitespace@5..6 " "
                    String@6..9 "\"G\""
                    Newline@9..10 "\n"
                  Whitespace@10..12 "  "
                  Indent@12..12 ""
                  Word@12..16 "note"
                  Colon@16..17 ":"
                  Newline@17..18 "\n"
                  MarkdownBlock@18..28 "    Text.\n"
                  Dedent@28..28 ""
            "#]],
        );
    }

    #[test]
    fn empty_input_is_an_empty_document() {
        let tree = parse("").unwrap().syntax();
        assert_eq!(tree.kind(), SyntaxKind::Document);
        assert_eq!(tree.children_with_tokens().count(), 0);
    }

    #[test]
    fn the_tree_reproduces_the_document() {
        check_round_trip("guide \"Franklin County Voters' Guide\"\ndate 2026-11-03\n");
    }

    #[test]
    fn the_tree_reproduces_errors() {
        check_round_trip("subtitle\t\"Vote for one\n12abc @ ~\n");
    }

    #[test]
    fn the_tree_reproduces_layout_and_markdown() {
        check_round_trip("ballot \"B\"\n  contest \"C\"\n    note:\n      Read *both*.\n\nend");
    }

    #[test]
    fn the_tree_reproduces_line_terminators_and_a_byte_order_mark() {
        check_round_trip("\u{FEFF}a\r\n  b\rc\n\n# comment\n");
    }

    #[test]
    fn ignored_tokens_at_the_end_are_inside_the_document() {
        check(
            "word\n\n# last\n",
            &expect![[r##"
                Document@0..13
                  Word@0..4 "word"
                  Newline@4..5 "\n"
                  BlankLine@5..6 "\n"
                  Comment@6..12 "# last"
                  BlankLine@12..13 "\n"
            "##]],
        );
    }

    #[test]
    fn a_document_of_only_ignored_tokens_keeps_them() {
        check(
            "# just a comment\n",
            &expect![[r##"
                Document@0..17
                  Comment@0..16 "# just a comment"
                  BlankLine@16..17 "\n"
            "##]],
        );
    }

    #[test]
    fn a_rejected_document_is_not_parsed() {
        assert_eq!(
            parse("a\0b"),
            Err(InputRejection::NulCharacter { offset: 1 })
        );
    }

    fn parser(text: &str) -> Parser<'_> {
        let mut parser = Parser::new(text, lex(text).unwrap().tokens);
        parser.start_node_here(SyntaxKind::Document);
        parser
    }

    fn tree(mut parser: Parser<'_>) -> SyntaxNode {
        parser.finish_node();
        SyntaxNode::new_root(parser.finish().0)
    }

    fn unexpected(offset: u32, len: u32, expected: TokenKind) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedToken,
            offset,
            len,
            expected: vec![Expected::Token(expected)],
        }
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
        let parser = parser("  # aside\n\nword\n");
        assert_eq!(parser.current(), Some(TokenKind::Word));
        assert_eq!(tree(parser).to_string(), "");
    }

    #[test]
    fn current_keyword_is_the_keyword_of_the_next_word() {
        assert_eq!(
            parser("# aside\ndate x").current_keyword(),
            Some(Keyword::Date)
        );
        assert_eq!(parser("dates").current_keyword(), None);
        assert_eq!(parser("\"date\"").current_keyword(), None);
        assert_eq!(parser("").current_keyword(), None);
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
    fn eat_skipped_takes_ignored_tokens_up_to_the_next_real_one() {
        let mut parser = parser("# aside\nword");
        parser.eat_skipped();
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

    #[test]
    fn expect_takes_the_token_when_it_is_there() {
        let mut parser = parser("1\n");
        assert!(parser.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(parser.current(), Some(TokenKind::Newline));
        assert_eq!(parser.errors, []);
    }

    #[test]
    fn expect_reports_a_missing_token_at_the_end_of_the_line() {
        let mut parser = parser("oval  # version?\n");
        parser.bump();
        assert!(!parser.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(parser.errors, [missing_integer(4)]);
    }

    #[test]
    fn expect_reports_a_missing_token_at_the_end_of_input() {
        let mut parser = parser("");
        assert!(!parser.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(parser.errors, [missing_integer(0)]);
    }

    #[test]
    fn expect_skips_the_rest_of_the_line_when_another_token_is_there() {
        let mut parser = parser("oval x y\n");
        parser.bump();
        assert!(!parser.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(parser.current(), Some(TokenKind::Newline));
        assert_eq!(parser.errors, [unexpected(5, 3, TokenKind::Integer)]);
    }

    #[test]
    fn current_skips_error_tokens() {
        let parser = parser("\t~word\n");
        assert_eq!(parser.current(), Some(TokenKind::Word));
    }

    #[test]
    fn a_line_of_only_error_tokens_is_skipped_like_a_blank_line() {
        let mut parser = parser("\t\nword\n");
        assert_eq!(parser.current(), Some(TokenKind::Word));
        parser.bump();
        assert_eq!(tree(parser).to_string(), "\t\nword");
    }

    #[test]
    fn a_line_with_another_token_keeps_its_newline() {
        let mut parser = parser("a\t\n");
        parser.bump();
        assert_eq!(parser.current(), Some(TokenKind::Newline));
    }

    #[test]
    fn a_line_that_opens_a_block_keeps_its_newline() {
        let mut parser = parser("a\n  \t\n");
        parser.bump();
        parser.bump();
        assert_eq!(parser.current(), Some(TokenKind::Indent));
        parser.bump();
        assert_eq!(parser.current(), Some(TokenKind::Newline));
    }

    #[test]
    fn expect_reports_nothing_on_a_line_with_an_error_token() {
        for text in ["oval 1x\n", "oval\t\n", "oval ~\n"] {
            let mut parser = parser(text);
            parser.bump();
            assert!(!parser.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
            assert_eq!(parser.errors, [], "{text:?}");
        }
    }

    #[test]
    fn an_error_token_on_an_earlier_line_does_not_stop_a_report() {
        let mut parser = parser("~\noval\n");
        parser.bump();
        assert!(!parser.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue));
        assert_eq!(parser.errors, [missing_integer(6)]);
    }

    #[test]
    fn a_skipped_line_is_an_error_node_up_to_the_end_of_the_line() {
        let mut parser = parser("oval x  # aside\n");
        parser.bump();
        parser.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue);
        parser.end_of_line();
        expect![[r##"
            Document@0..16
              Word@0..4 "oval"
              Whitespace@4..5 " "
              ErrorNode@5..6
                Word@5..6 "x"
              Whitespace@6..8 "  "
              Comment@8..15 "# aside"
              Newline@15..16 "\n"
        "##]]
        .assert_eq(&format!("{:#?}", tree(parser)));
    }

    #[test]
    fn a_skipped_line_still_reports_beside_an_error_token() {
        let mut parser = parser("oval ~ x\n");
        parser.bump();
        parser.expect(TokenKind::Integer, SyntaxErrorKind::MissingValue);
        assert_eq!(parser.errors, [unexpected(7, 1, TokenKind::Integer)]);
    }

    #[test]
    fn end_of_line_takes_the_newline() {
        let mut parser = parser("a\nb");
        parser.bump();
        parser.end_of_line();
        assert_eq!(parser.current(), Some(TokenKind::Word));
        assert_eq!(parser.errors, []);
    }

    #[test]
    fn end_of_line_skips_tokens_before_the_newline() {
        let mut parser = parser("a b c  # aside\nd");
        parser.bump();
        parser.end_of_line();
        assert_eq!(parser.current(), Some(TokenKind::Word));
        assert_eq!(parser.errors, [unexpected(2, 3, TokenKind::Newline)]);
    }
}
