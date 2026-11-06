//! Builds a lossless syntax tree from source text.

mod ballot;
mod contest;
mod document;
mod measure;
mod recommend;
#[cfg(test)]
mod tests;

use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

use crate::SyntaxKind::{
    self, At, Block, Colon, DateField, DateKeyword, DateLiteral, Dedent, DescriptionKeyword,
    ErrorNode, Identifier, IfNoKeyword, IfYesKeyword, Indent, JustificationKeyword, MarkdownBlock,
    MarkdownField, Newline, NoteKeyword, StringField, StringLiteral, UrlField, UrlLiteral,
    WebsiteKeyword, Word,
};

use crate::lexer::{InputRejection, LexicalError, Token, lex};
use crate::{Keyword, SyntaxNode};

/// An error found while parsing tokens into a syntax tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    /// The kinds that would have been accepted. Empty when nothing was acceptable.
    pub expected: Vec<SyntaxKind>,

    /// The kind actually found, or `None` at end of input.
    pub found: Option<SyntaxKind>,

    /// The skipped text, or an empty range where something was missing.
    pub range: TextRange,
}

/// Any error found while turning source text into a tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SyntaxError {
    /// The lexer could not tokenize some text.
    Lexical(LexicalError),

    /// The parser could not fit the tokens to the grammar.
    Parse(ParseError),
}

impl SyntaxError {
    /// The range of source text where the error occurred.
    pub fn range(&self) -> TextRange {
        match self {
            Self::Lexical(error) => error.range,
            Self::Parse(error) => error.range,
        }
    }
}

/// The result of parsing source text into a syntax tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    root: GreenNode,
    errors: Vec<SyntaxError>,
}

impl ParseResult {
    /// The root of the syntax tree.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.root.clone())
    }

    /// All errors, in source order.
    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }
}

/// Parses `source` into a tree, or rejects the input as a whole.
pub fn parse(source: &str) -> Result<ParseResult, InputRejection> {
    let lexed = lex(source)?;
    let mut parser = Parser {
        source,
        tokens: lexed.tokens,
        pos: 0,
        offset: 0,
        stances: 0,
        builder: GreenNodeBuilder::new(),
        errors: Vec::new(),
    };
    parser.document();
    let mut errors: Vec<SyntaxError> = lexed.errors.into_iter().map(SyntaxError::Lexical).collect();
    errors.extend(parser.errors.into_iter().map(SyntaxError::Parse));
    errors.sort_by_key(|error| error.range().start());
    Ok(ParseResult {
        root: parser.builder.finish(),
        errors,
    })
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    offset: usize,
    stances: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

impl Parser<'_> {
    fn declaration_header(&mut self) {
        self.identifier();
        self.expect_value(StringLiteral);
        self.end_of_line();
    }

    fn identifier(&mut self) -> bool {
        if !self.at(At) {
            self.error(&[At]);
            return false;
        }
        self.start_node(Identifier);
        self.bump();
        if self.at_word() {
            self.bump_as(Word);
        } else {
            self.error(&[Word]);
        }
        self.finish_node();
        true
    }

    fn at_word(&self) -> bool {
        self.current()
            .is_some_and(|kind| kind == Word || kind.is_keyword())
    }

    fn required_block(
        &mut self,
        fields: &[SyntaxKind],
        items: &[SyntaxKind],
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        if self.at(Indent) {
            self.block(fields, items, parse);
        } else {
            self.error(&[Indent]);
        }
    }

    fn block(
        &mut self,
        fields: &[SyntaxKind],
        items: &[SyntaxKind],
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        if !self.at(Indent) {
            return;
        }
        self.start_node(Block);
        self.bump();
        let mut remaining = fields.to_vec();
        while let Some(kind) = self.current() {
            if kind == Dedent {
                break;
            }
            if fields.contains(&kind) {
                self.field_once(kind, &mut remaining, parse);
            } else if !(items.contains(&kind) && parse(self, kind)) {
                let expected: Vec<SyntaxKind> = remaining.iter().chain(items).copied().collect();
                self.recover_item(&expected);
            }
        }
        self.expect(Dedent);
        self.finish_node();
    }

    fn field_once(
        &mut self,
        kind: SyntaxKind,
        remaining: &mut Vec<SyntaxKind>,
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        if remaining.contains(&kind) {
            remaining.retain(|k| *k != kind);
            parse(self, kind);
        } else {
            self.misplaced(kind, remaining, parse);
        }
    }

    fn misplaced(
        &mut self,
        kind: SyntaxKind,
        expected: &[SyntaxKind],
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        self.start_node(ErrorNode);
        let start = self.offset;
        parse(self, kind);
        self.finish_node();
        self.errors.push(ParseError {
            expected: expected.to_vec(),
            found: Some(kind),
            range: TextRange::new(text_size(start), text_size(self.offset)),
        });
    }

    fn simple_field(&mut self, kind: SyntaxKind) -> bool {
        let (node, value) = match kind {
            DateKeyword => (DateField, DateLiteral),
            WebsiteKeyword => (UrlField, UrlLiteral),
            NoteKeyword | JustificationKeyword | DescriptionKeyword | IfYesKeyword
            | IfNoKeyword => (MarkdownField, MarkdownBlock),
            _ => (StringField, StringLiteral),
        };
        self.start_node(node);
        self.bump();
        if value == MarkdownBlock {
            self.markdown_value();
        } else {
            self.expect_value(value);
            self.end_of_line();
        }
        self.finish_node();
        true
    }

    fn markdown_value(&mut self) {
        if !self.expect(Colon) {
            self.end_of_line();
            return;
        }
        if self.at_line_end() {
            self.end_of_line();
            if !self.eat(MarkdownBlock) {
                self.error(&[MarkdownBlock]);
            }
        } else {
            self.end_of_line();
        }
    }

    fn expect_value(&mut self, kind: SyntaxKind) {
        if self.eat(kind) {
            return;
        }
        if self.at_line_end() {
            self.error(&[kind]);
        } else {
            self.recover_line(&[kind]);
        }
    }

    fn end_of_line(&mut self) {
        if !self.at_line_end() {
            self.recover_line(&[Newline]);
        }
        self.expect(Newline);
    }

    fn at_line_end(&self) -> bool {
        matches!(self.current(), None | Some(Newline | Indent | Dedent))
    }

    fn recover_line(&mut self, expected: &[SyntaxKind]) {
        self.start_node(ErrorNode);
        let start = self.offset;
        let found = self.current();
        while !self.at_line_end() {
            self.bump();
        }
        self.finish_node();
        self.errors.push(ParseError {
            expected: expected.to_vec(),
            found,
            range: TextRange::new(text_size(start), text_size(self.offset)),
        });
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.error(&[kind]);
        false
    }

    fn error(&mut self, expected: &[SyntaxKind]) {
        let offset = self.tokens[self.pos..]
            .iter()
            .take_while(|token| token.kind.is_trivia())
            .map(|token| usize::from(token.len))
            .sum::<usize>()
            + self.offset;
        self.errors.push(ParseError {
            expected: expected.to_vec(),
            found: self.current(),
            range: TextRange::empty(text_size(offset)),
        });
    }

    fn current(&self) -> Option<SyntaxKind> {
        let mut offset = self.offset;
        for token in &self.tokens[self.pos..] {
            if !token.kind.is_trivia() {
                return Some(self.classify(*token, offset));
            }
            offset += usize::from(token.len);
        }
        None
    }

    fn classify(&self, token: Token, offset: usize) -> SyntaxKind {
        if token.kind != Word {
            return token.kind;
        }
        let text = &self.source[offset..offset + usize::from(token.len)];
        Keyword::from_text(text).map_or(Word, SyntaxKind::from)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == Some(kind)
    }

    fn bump(&mut self) {
        self.eat_trivia();
        self.bump_raw();
    }

    fn bump_raw(&mut self) {
        let token = self.tokens[self.pos];
        let kind = self.classify(token, self.offset);
        self.bump_raw_as(kind);
    }

    fn bump_as(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        self.bump_raw_as(kind);
    }

    fn bump_raw_as(&mut self, kind: SyntaxKind) {
        let token = self.tokens[self.pos];
        let end = self.offset + usize::from(token.len);
        self.builder
            .token(kind.into(), &self.source[self.offset..end]);
        self.offset = end;
        self.pos += 1;
    }

    fn eat_trivia(&mut self) {
        while self
            .tokens
            .get(self.pos)
            .is_some_and(|token| token.kind.is_trivia())
        {
            self.bump_raw();
        }
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        self.builder.start_node(kind.into());
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    fn recover_item(&mut self, expected: &[SyntaxKind]) {
        self.start_node(ErrorNode);
        let start = self.offset;
        let found = self.current();
        if self.at(Indent) {
            self.skip_block();
        } else {
            while !matches!(self.current(), None | Some(Newline | Indent | Dedent)) {
                self.bump();
            }
            if self.at(Newline) {
                self.bump();
            }
            if self.at(Indent) {
                self.skip_block();
            }
            if self.offset == start && self.current().is_some() {
                self.bump();
            }
        }
        self.finish_node();
        self.errors.push(ParseError {
            expected: expected.to_vec(),
            found,
            range: TextRange::new(text_size(start), text_size(self.offset)),
        });
    }

    fn skip_block(&mut self) {
        let mut depth = 0usize;
        while let Some(kind) = self.current() {
            self.bump();
            match kind {
                Indent => depth += 1,
                Dedent => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}

fn text_size(offset: usize) -> TextSize {
    TextSize::try_from(offset).expect("source longer than 4 GiB")
}
