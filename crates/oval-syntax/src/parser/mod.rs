//! Builds a lossless syntax tree from source text.
//!
//! [`parse`] lexes its input with [`crate::lexer`] and matches the tokens against the syntactic
//! grammar of Section 2 of the specification, building a [`rowan`] tree whose kinds are
//! [`SyntaxKind`]. The tree covers every byte of the source. Trivia, lexer errors, and text the
//! parser discards or sets aside all appear in it, the last two inside [`SyntaxKind::ErrorNode`]
//! nodes.
//!
//! The parser never stops at an error. Each syntax error of Section 3.2 is recorded as a
//! [`ParseError`] and recovered from as the specification prescribes, by skipping the line or the
//! entry, by assuming the missing token, or by setting the entry aside. The typed views in
//! [`crate::ast`] read the resulting tree whether or not it has errors.

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

/// The syntax errors of the specification, Section 3.2.
///
/// Each variant is one entry of that section; [`identifier`](Self::identifier) and
/// [`title`](Self::title) give the entry's identifier and title. The recovery named on each
/// variant is the one the specification prescribes, so two conformant parsers build the same tree
/// and report the same errors for the same broken document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseErrorKind {
    /// A token the grammar does not accept where the line has not yet ended; the line is skipped
    /// from that token on, and the construct keeps what was parsed before it.
    UnexpectedToken,
    /// An identifier is required and the line ends or a string follows where it should begin, or
    /// `@` is not followed by a word; the missing token is assumed.
    MissingIdentifier,
    /// A declaration's identifier is followed by the end of the line instead of its name; the
    /// missing token is assumed.
    MissingName,
    /// A contest's name is followed by the end of the line instead of a kind; the missing token
    /// is assumed and the contest is kept with no kind.
    MissingContestKind,
    /// A line ends where a value is required, such as after a field keyword or after `oval`; the
    /// missing token is assumed and the field is kept with no value.
    MissingValue,
    /// A Markdown field's line ends with `:` and the next line with content is not indented more
    /// than it; the missing token is assumed and the field is kept with an empty body.
    MissingBody,
    /// A construct whose block is required is not followed by an indented line; the missing token
    /// is assumed and the construct is kept with an empty block.
    MissingBlock,
    /// A line that takes no block is followed by one; the block is skipped and the line before it
    /// is kept.
    UnexpectedBlock,
    /// A line begins with a token that cannot begin any entry the enclosing block accepts; the
    /// entry is skipped together with its block, if any.
    UnexpectedEntry,
    /// A field keyword appears a second time in the same block or at the document level; the
    /// second entry is set aside and the first stands.
    RepeatedField,
    /// `incumbent` or `withdrawn` appears twice on one `campaign` line; the second flag is set
    /// aside.
    RepeatedFlag,
    /// A document-level entry appears after a group that must follow it has begun, such as a
    /// guide field after the first root ballot; the entry is set aside.
    MisplacedEntry,
    /// A contest's `recommend` has a block with neither `support` nor `oppose`; nothing is
    /// discarded and the error spans the keyword.
    EmptyRecommendation,
}

impl ParseErrorKind {
    /// Every kind, in the specification's order.
    pub const ALL: &'static [Self] = &[
        Self::UnexpectedToken,
        Self::MissingIdentifier,
        Self::MissingName,
        Self::MissingContestKind,
        Self::MissingValue,
        Self::MissingBody,
        Self::MissingBlock,
        Self::UnexpectedBlock,
        Self::UnexpectedEntry,
        Self::RepeatedField,
        Self::RepeatedFlag,
        Self::MisplacedEntry,
        Self::EmptyRecommendation,
    ];

    /// The identifier of this error's entry in the specification, such as `unexpected-token`.
    ///
    /// Identifiers are stable across editions, so tools report errors by identifier.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::UnexpectedToken => "unexpected-token",
            Self::MissingIdentifier => "missing-identifier",
            Self::MissingName => "missing-name",
            Self::MissingContestKind => "missing-contest-kind",
            Self::MissingValue => "missing-value",
            Self::MissingBody => "missing-body",
            Self::MissingBlock => "missing-block",
            Self::UnexpectedBlock => "unexpected-block",
            Self::UnexpectedEntry => "unexpected-entry",
            Self::RepeatedField => "repeated-field",
            Self::RepeatedFlag => "repeated-flag",
            Self::MisplacedEntry => "misplaced-entry",
            Self::EmptyRecommendation => "empty-recommendation",
        }
    }

    /// The title of this error's entry in the specification, such as `Unexpected Token`.
    pub const fn title(self) -> &'static str {
        match self {
            Self::UnexpectedToken => "Unexpected Token",
            Self::MissingIdentifier => "Missing Identifier",
            Self::MissingName => "Missing Name",
            Self::MissingContestKind => "Missing Contest Kind",
            Self::MissingValue => "Missing Value",
            Self::MissingBody => "Missing Body",
            Self::MissingBlock => "Missing Block",
            Self::UnexpectedBlock => "Unexpected Block",
            Self::UnexpectedEntry => "Unexpected Entry",
            Self::RepeatedField => "Repeated Field",
            Self::RepeatedFlag => "Repeated Flag",
            Self::MisplacedEntry => "Misplaced Entry",
            Self::EmptyRecommendation => "Empty Recommendation",
        }
    }
}

/// A syntax error, with its kind, what the parser expected and found, and the range it covers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    /// The error that occurred.
    pub kind: ParseErrorKind,

    /// The kinds the grammar would have accepted at that point. Empty when nothing was
    /// acceptable.
    pub expected: Vec<SyntaxKind>,

    /// The kind the parser found instead, or `None` at end of input.
    ///
    /// Words are classified, so a keyword appears as its keyword kind rather than as
    /// [`SyntaxKind::Word`].
    pub found: Option<SyntaxKind>,

    /// The range the specification assigns the error. It is the discarded or set-aside text, or
    /// an empty range where a missing token would begin.
    pub range: TextRange,
}

/// An error from either stage of turning source text into a tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SyntaxError {
    /// The lexer could not tokenize some text, or found a flaw in a token it kept.
    Lexical(LexicalError),

    /// The parser could not fit the tokens to the grammar.
    Parse(ParseError),
}

impl SyntaxError {
    /// The range of the error, whichever stage reported it.
    pub fn range(&self) -> TextRange {
        match self {
            Self::Lexical(error) => error.range,
            Self::Parse(error) => error.range,
        }
    }
}

/// The tree and the errors of one parse.
///
/// The tree is present whether or not there are errors; it covers every byte of the source, and
/// [`errors`](Self::errors) lists what went wrong. A result with no errors is a well-formed
/// document in the specification's sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    root: GreenNode,
    errors: Vec<SyntaxError>,
}

impl ParseResult {
    /// The root of the tree, a [`SyntaxKind::Document`] node whose text is the source.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.root.clone())
    }

    /// Every lexical and syntax error, in source order.
    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }
}

/// Parses `source` into a tree, or rejects the input as a whole.
///
/// Parsing never stops at an error. The tree of the result covers every byte of `source`, and
/// its errors list every lexical and syntax error in source order, each recovered from as
/// Sections 3.1 and 3.2 prescribe.
///
/// # Errors
///
/// Returns an [`InputRejection`] when `source` contains U+0000. No other input is rejected.
///
/// # Panics
///
/// Panics when `source` is longer than a [`TextSize`] can address, 4 GiB.
///
/// # Examples
///
/// ```
/// use oval_syntax::parser::{ParseErrorKind, SyntaxError, parse};
///
/// let source = "oval 1\nguide\n";
/// let result = parse(source).unwrap();
/// assert!(matches!(
///     result.errors(),
///     [SyntaxError::Parse(error)] if error.kind == ParseErrorKind::MissingValue
/// ));
/// assert_eq!(result.syntax().text().to_string(), source);
/// ```
pub fn parse(source: &str) -> Result<ParseResult, InputRejection> {
    let lexed = lex(source)?;
    let mut parser = Parser {
        source,
        tokens: lexed.tokens,
        pos: 0,
        offset: 0,
        line_start: 0,
        swallow_body: false,
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

/// The state of one pass over the token stream, building the tree as it goes.
struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    offset: usize,
    line_start: usize,
    swallow_body: bool,
    stances: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

fn is_skipped(kind: SyntaxKind) -> bool {
    kind.is_trivia() || kind == SyntaxKind::Error
}

impl Parser<'_> {
    fn declaration_header(&mut self) {
        self.identifier();
        self.expect_value_or(StringLiteral, ParseErrorKind::MissingName);
        self.end_of_line();
    }

    fn identifier(&mut self) -> bool {
        if !self.at(At) {
            if self.at_line_end() || self.at(StringLiteral) {
                self.missing(ParseErrorKind::MissingIdentifier, &[At]);
            } else {
                self.recover_line(&[At]);
            }
            return false;
        }
        self.start_node(Identifier);
        self.bump();
        if self.at_word() {
            self.bump_as(Word);
        } else {
            self.missing(ParseErrorKind::MissingIdentifier, &[Word]);
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
            self.error_at(self.offset, ParseErrorKind::MissingBlock, &[Indent]);
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
            if kind == Newline {
                self.bump();
            } else if fields.contains(&kind) {
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
            self.set_aside(ParseErrorKind::RepeatedField, kind, remaining, parse);
        }
    }

    fn set_aside(
        &mut self,
        error_kind: ParseErrorKind,
        kind: SyntaxKind,
        expected: &[SyntaxKind],
        parse: fn(&mut Self, SyntaxKind) -> bool,
    ) {
        self.start_node(ErrorNode);
        let start = self.offset;
        parse(self, kind);
        self.finish_node();
        self.errors.push(ParseError {
            kind: error_kind,
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
        if !self.expect_value(Colon) {
            self.end_of_line();
            return;
        }
        if self.at_line_end() {
            self.end_of_line();
            if !self.eat(MarkdownBlock) {
                self.error_at(self.offset, ParseErrorKind::MissingBody, &[MarkdownBlock]);
            }
        } else {
            self.end_of_line();
        }
    }

    fn expect_value(&mut self, kind: SyntaxKind) -> bool {
        self.expect_value_or(kind, ParseErrorKind::MissingValue)
    }

    fn expect_value_or(&mut self, kind: SyntaxKind, missing: ParseErrorKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        if self.at_line_end() {
            self.missing(missing, &[kind]);
        } else {
            self.recover_line(&[kind]);
        }
        false
    }

    fn end_of_line(&mut self) {
        if !self.at_line_end() {
            self.recover_line(&[Newline]);
        }
        self.expect(Newline);
        if self.swallow_body {
            self.swallow_body = false;
            if self.at(MarkdownBlock) {
                self.start_node(ErrorNode);
                self.bump();
                self.finish_node();
            }
        }
    }

    fn at_line_end(&self) -> bool {
        matches!(self.current(), None | Some(Newline | Indent | Dedent))
    }

    fn skip_token(&mut self) {
        self.start_node(ErrorNode);
        self.bump();
        self.finish_node();
    }

    fn recover_line(&mut self, expected: &[SyntaxKind]) {
        self.start_node(ErrorNode);
        let start = self.offset;
        let found = self.current();
        while !self.at_line_end() {
            self.bump();
        }
        self.finish_node();
        self.swallow_body = true;
        self.errors.push(ParseError {
            kind: ParseErrorKind::UnexpectedToken,
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
        self.missing(ParseErrorKind::MissingValue, &[kind]);
        false
    }

    fn missing(&mut self, kind: ParseErrorKind, expected: &[SyntaxKind]) {
        if self.line_contains_error() {
            return;
        }
        let offset = self.tokens[self.pos..]
            .iter()
            .take_while(|token| is_skipped(token.kind))
            .map(|token| usize::from(token.len))
            .sum::<usize>()
            + self.offset;
        self.error_at(offset, kind, expected);
    }

    fn error_at(&mut self, offset: usize, kind: ParseErrorKind, expected: &[SyntaxKind]) {
        self.errors.push(ParseError {
            kind,
            expected: expected.to_vec(),
            found: self.current(),
            range: TextRange::empty(text_size(offset)),
        });
    }

    fn line_contains_error(&self) -> bool {
        self.tokens[self.line_start..]
            .iter()
            .take_while(|token| token.kind != Newline)
            .any(|token| token.kind == SyntaxKind::Error)
    }

    fn current(&self) -> Option<SyntaxKind> {
        let mut offset = self.offset;
        for token in &self.tokens[self.pos..] {
            if !is_skipped(token.kind) {
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
        self.eat_skipped();
        self.bump_raw();
    }

    fn bump_raw(&mut self) {
        let token = self.tokens[self.pos];
        let kind = self.classify(token, self.offset);
        self.bump_raw_as(kind);
    }

    fn bump_as(&mut self, kind: SyntaxKind) {
        self.eat_skipped();
        self.bump_raw_as(kind);
    }

    fn bump_raw_as(&mut self, kind: SyntaxKind) {
        let token = self.tokens[self.pos];
        let end = self.offset + usize::from(token.len);
        self.builder
            .token(kind.into(), &self.source[self.offset..end]);
        self.offset = end;
        self.pos += 1;
        if matches!(token.kind, Newline | MarkdownBlock) {
            self.line_start = self.pos;
        }
    }

    fn eat_skipped(&mut self) {
        while self
            .tokens
            .get(self.pos)
            .is_some_and(|token| is_skipped(token.kind))
        {
            self.bump_raw();
        }
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        self.eat_skipped();
        self.builder.start_node(kind.into());
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    fn recover_item(&mut self, expected: &[SyntaxKind]) {
        self.start_node(ErrorNode);
        let start = self.offset;
        let found = self.current();
        let kind = if self.at(Indent) {
            ParseErrorKind::UnexpectedBlock
        } else {
            ParseErrorKind::UnexpectedEntry
        };
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
            kind,
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
