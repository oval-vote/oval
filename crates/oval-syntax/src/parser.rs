//! Builds a lossless syntax tree from source text.

use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

use crate::SyntaxKind::{self, Dedent, Document, ErrorNode, Indent, Newline};
use crate::SyntaxNode;
use crate::lexer::{LexError, Token, lex};

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
    Lex(LexError),

    /// The parser could not fit the tokens to the grammar.
    Parse(ParseError),
}

impl SyntaxError {
    /// The range of source text where the error occurred.
    pub fn range(&self) -> TextRange {
        match self {
            Self::Lex(error) => error.range,
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

/// Parses `source` into a tree.
pub fn parse(source: &str) -> ParseResult {
    let lexed = lex(source);
    let mut parser = Parser {
        source,
        tokens: lexed.tokens,
        pos: 0,
        offset: 0,
        builder: GreenNodeBuilder::new(),
        errors: Vec::new(),
    };
    parser.document();
    let mut errors: Vec<SyntaxError> = lexed.errors.into_iter().map(SyntaxError::Lex).collect();
    errors.extend(parser.errors.into_iter().map(SyntaxError::Parse));
    errors.sort_by_key(|error| error.range().start());
    ParseResult {
        root: parser.builder.finish(),
        errors,
    }
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    offset: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

impl Parser<'_> {
    fn document(&mut self) {
        self.builder.start_node(Document.into());
        while self.current().is_some() {
            self.recover_item(&[]);
        }
        self.eat_trivia();
        self.finish_node();
    }

    fn current(&self) -> Option<SyntaxKind> {
        self.tokens[self.pos..]
            .iter()
            .map(|token| token.kind)
            .find(|kind| !kind.is_trivia())
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
        let end = self.offset + usize::from(token.len);
        self.builder
            .token(token.kind.into(), &self.source[self.offset..end]);
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
            self.block();
        } else {
            while !matches!(self.current(), None | Some(Newline | Indent | Dedent)) {
                self.bump();
            }
            if self.at(Newline) {
                self.bump();
            }
            if self.at(Indent) {
                self.block();
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

    fn block(&mut self) {
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

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use rowan::{NodeOrToken, TextRange};

    use super::{ParseError, SyntaxError, parse};
    use crate::SyntaxKind;
    use crate::lexer::LexErrorReason;

    const SAMPLE: &str = "\
guide \"Franklin Voter Guide\"
election \"November 3, 2026 General Election\"
date 2026-11-03

note:
  Written by the Franklin Civic Alliance.

  Second paragraph.

party @dem \"Democratic\"
  abbreviation \"D\"

ballot @franklin \"State of Franklin\"
  section \"Statewide Offices\"
    contest @gov \"Governor\" single
      campaign incumbent @jlee \"Jordan Lee\"
        party @dem
        website https://leepatel2026.example
      campaign @rkim \"Riley Kim\" # trailing comment
      recommend
        support @jlee
        oppose @rkim
        justification:
          Lee delivered.
  measure @prop1 \"Proposition 1\"
    supporters \"A\", \"B\"
    opponents none
    recommend yes
";

    fn dump(node: &crate::SyntaxNode) -> String {
        fn go(element: rowan::SyntaxElement<crate::OvalLanguage>, depth: usize, out: &mut String) {
            let indent = "  ".repeat(depth);
            match element {
                NodeOrToken::Node(node) => {
                    writeln!(out, "{indent}{:?}", node.kind()).unwrap();
                    for child in node.children_with_tokens() {
                        go(child, depth + 1, out);
                    }
                }
                NodeOrToken::Token(token) => {
                    if !token.kind().is_trivia() {
                        writeln!(out, "{indent}{:?} {:?}", token.kind(), token.text()).unwrap();
                    }
                }
            }
        }
        let mut out = String::new();
        go(NodeOrToken::Node(node.clone()), 0, &mut out);
        out
    }

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    #[test]
    fn the_tree_reproduces_the_source_exactly() {
        for source in [
            "",
            "   ",
            "\n\n",
            "# only a comment",
            "guide \"x\"",
            "guide \"x\"\n",
            "  indented first line\n",
            "a\n  b\n    c\nd",
            "note:\n  body\n\n  more\n",
            "\u{FEFF}guide \"x\"\r\n\tbad\r\n",
            "\"unterminated\n@9 12abc https://\n",
            SAMPLE,
        ] {
            let parse = parse(source);
            assert_eq!(parse.syntax().text(), source, "round trip of {source:?}");
            assert_eq!(parse.syntax().kind(), SyntaxKind::Document);
        }
    }

    #[test]
    fn every_line_is_an_error_item_until_productions_exist() {
        let parse = parse("guide \"x\"\nparty @d \"D\"\n");
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  ErrorNode
    GuideKeyword \"guide\"
    StringLiteral \"\\\"x\\\"\"
    Newline \"\\n\"
  ErrorNode
    PartyKeyword \"party\"
    Identifier \"@d\"
    StringLiteral \"\\\"D\\\"\"
    Newline \"\\n\"
"
        );
        assert_eq!(
            parse.errors(),
            [
                SyntaxError::Parse(ParseError {
                    expected: vec![],
                    found: Some(SyntaxKind::GuideKeyword),
                    range: range(0, 10),
                }),
                SyntaxError::Parse(ParseError {
                    expected: vec![],
                    found: Some(SyntaxKind::PartyKeyword),
                    range: range(10, 23),
                }),
            ]
        );
    }

    #[test]
    fn an_error_item_swallows_its_own_block_and_nothing_more() {
        let source = "contest @x\n  campaign @a\n  recommend\n    support @a\nmeasure @m\n";
        let parse = parse(source);
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  ErrorNode
    ContestKeyword \"contest\"
    Identifier \"@x\"
    Newline \"\\n\"
    Indent \"\"
    CampaignKeyword \"campaign\"
    Identifier \"@a\"
    Newline \"\\n\"
    RecommendKeyword \"recommend\"
    Newline \"\\n\"
    Indent \"\"
    SupportKeyword \"support\"
    Identifier \"@a\"
    Newline \"\\n\"
    Dedent \"\"
    Dedent \"\"
  ErrorNode
    MeasureKeyword \"measure\"
    Identifier \"@m\"
    Newline \"\\n\"
"
        );
        assert_eq!(parse.errors().len(), 2);
        assert_eq!(parse.errors()[0].range(), range(0, 52));
        assert_eq!(parse.errors()[1].range(), range(52, 63));
    }

    #[test]
    fn an_indented_first_line_is_one_error_item() {
        let parse = parse("  yes\n  no\nyes");
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  ErrorNode
    Indent \"\"
    YesKeyword \"yes\"
    Newline \"\\n\"
    NoKeyword \"no\"
    Newline \"\\n\"
    Dedent \"\"
  ErrorNode
    YesKeyword \"yes\"
    Newline \"\"
"
        );
    }

    #[test]
    fn trivia_is_attached_to_the_enclosing_node() {
        let parse = parse("# lead\n\nyes # trail\n\n# tail\n");
        let root = parse.syntax();
        let error_node = root.first_child().expect("one error item");
        assert_eq!(error_node.text(), "yes # trail\n");
        assert_eq!(root.text(), "# lead\n\nyes # trail\n\n# tail\n");
    }

    #[test]
    fn lexer_errors_are_forwarded_in_source_order() {
        let parse = parse("\tyes\n@9\n");
        let kinds: Vec<&str> = parse
            .errors()
            .iter()
            .map(|error| match error {
                SyntaxError::Lex(e) if e.reason == LexErrorReason::IllegalTab => "tab",
                SyntaxError::Lex(e) if e.reason == LexErrorReason::MalformedIdentifier => "ident",
                SyntaxError::Lex(_) => "other lex",
                SyntaxError::Parse(_) => "parse",
            })
            .collect();
        assert_eq!(kinds, ["tab", "parse", "ident", "parse"]);
    }
}
