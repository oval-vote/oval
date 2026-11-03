//! Builds a lossless syntax tree from source text.

use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

use crate::SyntaxKind::{
    self, BallotKeyword, CandidateKeyword, Colon, DateField, DateKeyword, DateLiteral, Dedent,
    Document, ElectionKeyword, ErrorNode, GuideKeyword, Indent, MarkdownBlock, MarkdownField,
    Newline, NoteKeyword, PartyKeyword, StringField, StringLiteral,
};
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
        let mut allowed = vec![
            GuideKeyword,
            ElectionKeyword,
            DateKeyword,
            NoteKeyword,
            PartyKeyword,
            CandidateKeyword,
            BallotKeyword,
        ];
        while let Some(kind) = self.current() {
            let field = match kind {
                GuideKeyword | ElectionKeyword => Some((StringField, StringLiteral)),
                DateKeyword => Some((DateField, DateLiteral)),
                NoteKeyword => Some((MarkdownField, MarkdownBlock)),
                _ => None,
            };
            match field {
                Some(_) if !allowed.contains(&kind) => self.duplicate_field(kind, &allowed),
                Some((node, value)) => {
                    allowed.retain(|k| *k != kind);
                    self.field(node, value);
                }
                None => self.recover_item(&allowed),
            }
        }
        self.eat_trivia();
        self.finish_node();
    }

    fn field(&mut self, node: SyntaxKind, value: SyntaxKind) {
        self.start_node(node);
        self.bump();
        if value == MarkdownBlock {
            self.markdown_value();
        } else {
            self.expect_value(value);
            self.end_of_line();
        }
        self.finish_node();
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

    fn duplicate_field(&mut self, kind: SyntaxKind, allowed: &[SyntaxKind]) {
        self.start_node(ErrorNode);
        let start = self.offset;
        let (node, value) = match kind {
            DateKeyword => (DateField, DateLiteral),
            NoteKeyword => (MarkdownField, MarkdownBlock),
            _ => (StringField, StringLiteral),
        };
        self.field(node, value);
        self.finish_node();
        self.errors.push(ParseError {
            expected: allowed.to_vec(),
            found: Some(kind),
            range: TextRange::new(text_size(start), text_size(self.offset)),
        });
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

    const TOP_LEVEL: [SyntaxKind; 7] = [
        SyntaxKind::GuideKeyword,
        SyntaxKind::ElectionKeyword,
        SyntaxKind::DateKeyword,
        SyntaxKind::NoteKeyword,
        SyntaxKind::PartyKeyword,
        SyntaxKind::CandidateKeyword,
        SyntaxKind::BallotKeyword,
    ];

    #[test]
    fn guide_fields_from_the_spec_example() {
        let source = "\
guide \"Franklin Voter Guide\"
election \"November 3, 2026 General Election\"
date 2026-11-03

note:
  Written by the Franklin Civic Alliance.
";
        let parse = parse(source);
        assert_eq!(parse.errors(), []);
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  StringField
    GuideKeyword \"guide\"
    StringLiteral \"\\\"Franklin Voter Guide\\\"\"
    Newline \"\\n\"
  StringField
    ElectionKeyword \"election\"
    StringLiteral \"\\\"November 3, 2026 General Election\\\"\"
    Newline \"\\n\"
  DateField
    DateKeyword \"date\"
    DateLiteral \"2026-11-03\"
    Newline \"\\n\"
  MarkdownField
    NoteKeyword \"note\"
    Colon \":\"
    Newline \"\\n\"
    MarkdownBlock \"  Written by the Franklin Civic Alliance.\\n\"
"
        );
    }

    #[test]
    fn guide_fields_may_appear_in_any_order() {
        let parse = parse("date 2026-11-03\nnote:\n  x\nguide \"g\"\nelection \"e\"");
        assert_eq!(parse.errors(), []);
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::DateField,
                SyntaxKind::MarkdownField,
                SyntaxKind::StringField,
                SyntaxKind::StringField
            ]
        );
    }

    #[test]
    fn a_missing_value_is_reported_where_it_should_be() {
        let parse = parse("guide\n");
        assert_eq!(
            dump(&parse.syntax()),
            "Document\n  StringField\n    GuideKeyword \"guide\"\n    Newline \"\\n\"\n"
        );
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::StringLiteral],
                found: Some(SyntaxKind::Newline),
                range: range(5, 5),
            })]
        );
    }

    #[test]
    fn a_wrong_value_is_one_error_covering_the_rest_of_the_line() {
        let parse = parse("guide 2026-11-03 yes\n");
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  StringField
    GuideKeyword \"guide\"
    ErrorNode
      DateLiteral \"2026-11-03\"
      YesKeyword \"yes\"
    Newline \"\\n\"
"
        );
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::StringLiteral],
                found: Some(SyntaxKind::DateLiteral),
                range: range(6, 20),
            })]
        );
    }

    #[test]
    fn extra_tokens_after_a_value_are_an_error_node_inside_the_field() {
        let parse = parse("guide \"x\" \"y\"\n");
        assert_eq!(
            dump(&parse.syntax()),
            "\
Document
  StringField
    GuideKeyword \"guide\"
    StringLiteral \"\\\"x\\\"\"
    ErrorNode
      StringLiteral \"\\\"y\\\"\"
    Newline \"\\n\"
"
        );
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::Newline],
                found: Some(SyntaxKind::StringLiteral),
                range: range(10, 13),
            })]
        );
    }

    #[test]
    fn a_repeated_field_is_an_error_node_and_names_what_was_still_allowed() {
        let parse = parse("guide \"a\"\ndate 2026-11-03\nguide \"b\"\n");
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::StringField,
                SyntaxKind::DateField,
                SyntaxKind::ErrorNode
            ]
        );
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![
                    SyntaxKind::ElectionKeyword,
                    SyntaxKind::NoteKeyword,
                    SyntaxKind::PartyKeyword,
                    SyntaxKind::CandidateKeyword,
                    SyntaxKind::BallotKeyword,
                ],
                found: Some(SyntaxKind::GuideKeyword),
                range: range(26, 36),
            })]
        );
    }

    #[test]
    fn a_markdown_field_needs_a_colon_a_newline_and_a_body() {
        let result = parse("note\n");
        assert_eq!(result.errors()[0].range(), range(4, 4));
        assert!(matches!(
            &result.errors()[0],
            SyntaxError::Parse(e) if e.expected == [SyntaxKind::Colon] && e.found == Some(SyntaxKind::Newline)
        ));

        let result = parse("note: yes\n");
        assert_eq!(
            dump(&result.syntax()),
            "\
Document
  MarkdownField
    NoteKeyword \"note\"
    Colon \":\"
    ErrorNode
      YesKeyword \"yes\"
    Newline \"\\n\"
"
        );
        assert_eq!(result.errors().len(), 1);

        let result = parse("note:\nguide \"g\"\n");
        assert_eq!(
            result.errors(),
            [SyntaxError::Parse(ParseError {
                expected: vec![SyntaxKind::MarkdownBlock],
                found: Some(SyntaxKind::GuideKeyword),
                range: range(6, 6),
            })]
        );
        let kinds: Vec<SyntaxKind> = result.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(kinds, [SyntaxKind::MarkdownField, SyntaxKind::StringField]);
    }

    #[test]
    fn an_unknown_top_level_line_names_everything_that_could_start_one() {
        let parse = parse("party @d \"D\"\n");
        assert_eq!(
            parse.errors(),
            [SyntaxError::Parse(ParseError {
                expected: TOP_LEVEL.to_vec(),
                found: Some(SyntaxKind::PartyKeyword),
                range: range(0, 13),
            })]
        );
    }

    #[test]
    fn an_unexpected_block_after_a_field_is_one_error() {
        let parse = parse("guide \"x\"\n  note:\n    body\n");
        let kinds: Vec<SyntaxKind> = parse.syntax().children().map(|n| n.kind()).collect();
        assert_eq!(kinds, [SyntaxKind::StringField, SyntaxKind::ErrorNode]);
        assert_eq!(parse.errors().len(), 1);
        assert!(
            matches!(&parse.errors()[0], SyntaxError::Parse(e) if e.found == Some(SyntaxKind::Indent))
        );
    }

    #[test]
    fn impossible_dates_are_left_to_validation() {
        assert_eq!(parse("date 2026-13-45\n").errors(), []);
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
