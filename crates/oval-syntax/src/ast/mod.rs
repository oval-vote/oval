//! Typed views over the syntax tree.
//!
//! Each view wraps a [`SyntaxNode`] of one kind and exposes the parts of that construct which the
//! specification names. There is [`Document`] for the root, [`Ballot`] and [`Section`] for the
//! tree of questions, [`Contest`], [`Campaign`], and [`Member`] for offices, [`Measure`] for
//! yes-or-no questions, and the field types for the values written under them. Every accessor is
//! total. A part the parser did not find is `None` or an empty iterator, since a document with
//! errors still has a tree and tools still read it.
//!
//! [`AstNode`] is the trait the views share. [`AstNode::cast`] turns a node into a view and
//! [`AstNode::syntax`] returns the node. A view is the node and nothing more, so it clones cheaply
//! and reads the tree each time an accessor is called.
//!
//! # Examples
//!
//! ```
//! use oval_syntax::ast::{AstNode, BallotItem, Document};
//! use oval_syntax::parser::parse;
//!
//! let source = r#"oval 1
//! guide "G"
//! election "E"
//! date 2026-11-03
//! ballot @city "City"
//!   contest @mayor "Mayor" single
//!     campaign @delgado "Rosa Delgado"
//!     recommend
//!       support @delgado
//! "#;
//! let result = parse(source).unwrap();
//! let document = Document::cast(result.syntax()).unwrap();
//! let ballot = document.ballots().next().unwrap();
//! let Some(BallotItem::Contest(contest)) = ballot.items().next() else {
//!     panic!("the first item is a contest");
//! };
//! assert_eq!(contest.title().as_deref(), Some("Mayor"));
//! assert_eq!(contest.campaigns().count(), 1);
//! ```

mod ballot;
mod contest;
mod document;
mod measure;
mod recommend;
#[cfg(test)]
mod tests;

use rowan::TextRange;

pub use ballot::{Ballot, BallotItem, Section, SectionItem};
pub use contest::{Campaign, Contest, ContestKind, Member};
pub use document::{Candidate, Document, Party, PartyReference, VersionMarker};
pub use measure::{Measure, MeasureRecommend, Position, StringListField};
pub use recommend::{ContestRecommend, Oppose, Support};

use crate::{Keyword, SyntaxKind, SyntaxNode, SyntaxToken, TextSize};

/// A typed view over a [`SyntaxNode`] of one particular kind.
///
/// Every view in this module implements it. The three required methods are the whole contract;
/// the provided ones read positions and structure that every construct has.
pub trait AstNode: Sized {
    /// Whether nodes of `kind` can be viewed as this type.
    fn can_cast(kind: SyntaxKind) -> bool;

    /// Wraps `node` if it has the right kind.
    ///
    /// `None` when [`can_cast`](Self::can_cast) is false for the node's kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use oval_syntax::ast::{AstNode, Document, Party};
    /// use oval_syntax::parser::parse;
    ///
    /// let result = parse("oval 1\nparty @dem \"Democratic\"\n").unwrap();
    /// assert!(Party::cast(result.syntax()).is_none());
    /// let document = Document::cast(result.syntax()).unwrap();
    /// assert_eq!(document.parties().count(), 1);
    /// ```
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// The node this view wraps.
    fn syntax(&self) -> &SyntaxNode;

    /// The source range the node covers.
    ///
    /// A construct with a block covers the block too, so the range of a ballot spans every question
    /// in it. [`header_line_range`](Self::header_line_range) is the first line alone.
    fn range(&self) -> TextRange {
        self.syntax().text_range()
    }

    /// The range of the node's first line, up to its line terminator.
    ///
    /// For a construct with a block this is the header line. It covers the keyword, the
    /// identifier, the name, and whatever else precedes the block. Diagnostics about a construct
    /// as a whole point here.
    fn header_line_range(&self) -> TextRange {
        let node = self.syntax();
        let end = tokens(node)
            .find(|token| token.kind() == SyntaxKind::Newline)
            .map_or_else(
                || node.text_range().end(),
                |token| token.text_range().start(),
            );
        TextRange::new(node.text_range().start(), end)
    }

    /// Whether an indented block follows the node's first line.
    ///
    /// `false` when the construct ends at its first line, whether or not the grammar allows that.
    /// A required block that is missing is the syntax error `missing-block`, and the tree records
    /// nothing in its place.
    fn has_block(&self) -> bool {
        block(self.syntax()).is_some()
    }
}

macro_rules! ast_node {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name($crate::SyntaxNode);

        impl $crate::ast::AstNode for $name {
            fn can_cast(kind: $crate::SyntaxKind) -> bool {
                kind == $crate::SyntaxKind::$name
            }

            fn cast(node: $crate::SyntaxNode) -> Option<Self> {
                Self::can_cast(node.kind()).then(|| Self(node))
            }

            fn syntax(&self) -> &$crate::SyntaxNode {
                &self.0
            }
        }
    };
}
pub(crate) use ast_node;

ast_node! {
    /// An identifier, written as `@` followed by a word.
    ///
    /// Identifiers name parties, candidates, ballots, contests, and measures, and share one
    /// namespace (Section 2.7). The same node is a declaration or a reference depending on where it
    /// is written; the construct around it says which.
    Identifier
}

impl Identifier {
    /// The name after the `@`.
    ///
    /// Empty when the parser recovered from an `@` with no word after it.
    pub fn name(&self) -> String {
        token(&self.0, SyntaxKind::Word).map_or_else(String::new, |word| word.text().to_owned())
    }

    /// The range of the name alone, without the `@`.
    ///
    /// Empty at the end of the identifier when the parser recovered from an `@` with no word after
    /// it. A tool that renames an identifier replaces this range.
    pub fn name_range(&self) -> TextRange {
        token(&self.0, SyntaxKind::Word).map_or_else(
            || TextRange::empty(self.0.text_range().end()),
            |word| word.text_range(),
        )
    }
}

/// A calendar date as written, with no check that it exists.
///
/// The lexer matches the shape `YYYY-MM-DD` only, so `2026-02-30` is a [`Date`]. Whether a date is
/// on the calendar is the validation rule `invalid-date`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date {
    /// The year, four digits.
    pub year: u16,
    /// The month as written, 1 through 12 when valid.
    pub month: u8,
    /// The day of the month as written, 1 through 31 when valid.
    pub day: u8,
}

ast_node! {
    /// A field whose value is a string, such as `guide`, `subtitle`, or `designation`.
    StringField
}

ast_node! {
    /// A field whose value is a date, which is to say `date`.
    DateField
}

ast_node! {
    /// A field whose value is a URL, which is to say `website`.
    UrlField
}

ast_node! {
    /// A field whose value is a Markdown body, which is to say `note`, `justification`,
    /// `description`, `if_yes`, or `if_no`.
    MarkdownField
}

impl StringField {
    /// The keyword that names the field, such as [`Keyword::Guide`] or [`Keyword::Subtitle`].
    ///
    /// The parser begins every field with its keyword, so this is `None` only for a node built
    /// some other way.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The string with its escapes resolved.
    ///
    /// Only `\"` and `\\` are escapes (Section 2.2); any other backslash is kept as written.
    /// `None` when the line ended before the value.
    pub fn value(&self) -> Option<String> {
        token(&self.0, SyntaxKind::StringLiteral).map(|t| string_value(t.text()))
    }
}

impl DateField {
    /// The keyword that names the field, which is [`Keyword::Date`].
    ///
    /// The parser begins every field with its keyword, so this is `None` only for a node built
    /// some other way.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The date as written.
    ///
    /// The parts are split from the text without validation, so a month of 13 comes back as 13.
    /// `None` when the line ended before the value.
    pub fn value(&self) -> Option<Date> {
        token(&self.0, SyntaxKind::DateLiteral).map(|t| date_value(t.text()))
    }

    /// The range of the date itself, without the keyword.
    ///
    /// `None` when the line ended before the value.
    pub fn value_range(&self) -> Option<TextRange> {
        token_range(&self.0, SyntaxKind::DateLiteral)
    }
}

impl UrlField {
    /// The keyword that names the field, which is [`Keyword::Website`].
    ///
    /// The parser begins every field with its keyword, so this is `None` only for a node built
    /// some other way.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The URL text, exactly as written.
    ///
    /// `None` when the line ended before the value.
    pub fn value(&self) -> Option<String> {
        token(&self.0, SyntaxKind::UrlLiteral).map(|t| t.text().to_owned())
    }

    /// The range of the URL itself, without the keyword.
    ///
    /// `None` when the line ended before the value.
    pub fn value_range(&self) -> Option<TextRange> {
        token_range(&self.0, SyntaxKind::UrlLiteral)
    }
}

impl MarkdownField {
    /// The keyword that names the field, such as [`Keyword::Note`] or [`Keyword::Justification`].
    ///
    /// The parser begins every field with its keyword, so this is `None` only for a node built
    /// some other way.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The body with its common indentation removed, line terminators normalized to `\n`,
    /// and no trailing terminator.
    ///
    /// This is the value Section 2.2 derives from a `MarkdownBlock` token. `None` when no body
    /// followed the colon, which the parser reports as `missing-body`.
    pub fn value(&self) -> Option<String> {
        self.body().map(|body| body.value)
    }

    /// The body's value together with the mapping back to its source positions.
    ///
    /// Tools that report on the Markdown, such as the `markdown-profile` check, use this to place
    /// a diagnostic in the source. `None` when no body followed the colon.
    pub fn body(&self) -> Option<MarkdownBody> {
        token(&self.0, SyntaxKind::MarkdownBlock)
            .map(|t| MarkdownBody::new(t.text(), t.text_range().start()))
    }
}

/// The value of a Markdown body and where each of its lines sits in the source.
///
/// The value differs from the source text. Indentation is removed and line terminators are
/// normalized. [`source_offset`](Self::source_offset) and [`source_range`](Self::source_range)
/// map positions in the value back to the source so that a diagnostic found in the value can be
/// reported where the author wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownBody {
    value: String,
    start: TextSize,
    lines: Vec<(usize, TextSize)>,
}

impl MarkdownBody {
    fn new(text: &str, start: TextSize) -> Self {
        let lines = crate::text::lines_with_offsets(text);
        let indentation = lines
            .iter()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(_, line)| line.len() - line.trim_start_matches(' ').len())
            .min()
            .unwrap_or(0);
        let mut value = String::with_capacity(text.len());
        let mut mapping = Vec::with_capacity(lines.len());
        for (i, (offset, line)) in lines.iter().enumerate() {
            if i > 0 {
                value.push('\n');
            }
            let content = if line.trim().is_empty() {
                ""
            } else {
                &line[indentation..]
            };
            let source = start
                + u32::try_from(offset + line.len() - content.len())
                    .map_or(TextSize::from(u32::MAX), TextSize::from);
            mapping.push((value.len(), source));
            value.push_str(content);
        }
        Self {
            value,
            start,
            lines: mapping,
        }
    }

    /// The body with its common indentation removed, line terminators normalized to `\n`,
    /// and no trailing terminator.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The source offset of the byte at `value_offset` in the value.
    ///
    /// Offsets within a line map exactly. An offset at a line's `\n` maps to the source position
    /// after that line's content, which is where its terminator begins.
    pub fn source_offset(&self, value_offset: usize) -> TextSize {
        let (line_start, source) = self
            .lines
            .iter()
            .rev()
            .find(|(line_start, _)| *line_start <= value_offset)
            .copied()
            .unwrap_or((0, self.start));
        source
            + u32::try_from(value_offset - line_start)
                .map_or(TextSize::from(u32::MAX), TextSize::from)
    }

    /// The source range of the bytes in `range` of the value.
    ///
    /// Each end maps as [`source_offset`](Self::source_offset) maps it, so a range that spans
    /// lines covers the removed indentation and the original terminators between them.
    pub fn source_range(&self, range: std::ops::Range<usize>) -> TextRange {
        TextRange::new(
            self.source_offset(range.start),
            self.source_offset(range.end),
        )
    }
}

fn keyword(node: &SyntaxNode) -> Option<Keyword> {
    node.children_with_tokens()
        .filter_map(rowan::NodeOrToken::into_token)
        .find_map(|token| token.kind().as_keyword())
}

fn child<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    node.children().find_map(N::cast)
}

fn children<N: AstNode>(node: &SyntaxNode) -> impl Iterator<Item = N> + use<N> {
    node.children().filter_map(N::cast)
}

fn tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + use<> {
    node.children_with_tokens()
        .filter_map(rowan::NodeOrToken::into_token)
}

fn token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    tokens(node).find(|token| token.kind() == kind)
}

fn has_token(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    token(node, kind).is_some()
}

fn token_range(node: &SyntaxNode, kind: SyntaxKind) -> Option<TextRange> {
    token(node, kind).map(|token| token.text_range())
}

fn string_range(node: &SyntaxNode) -> Option<TextRange> {
    token_range(node, SyntaxKind::StringLiteral)
}

fn string(node: &SyntaxNode) -> Option<String> {
    token(node, SyntaxKind::StringLiteral).map(|t| string_value(t.text()))
}

fn integer(node: &SyntaxNode) -> Option<u32> {
    token(node, SyntaxKind::IntegerLiteral).and_then(|t| t.text().parse().ok())
}

fn block_children<N: AstNode>(node: &SyntaxNode) -> impl Iterator<Item = N> + use<N> {
    block(node)
        .into_iter()
        .flat_map(|b| b.children().filter_map(N::cast))
}

fn identifier(node: &SyntaxNode) -> Option<Identifier> {
    identifiers(node).next()
}

fn identifiers(node: &SyntaxNode) -> impl Iterator<Item = Identifier> + use<> {
    children::<Identifier>(node)
        .filter(|identifier| has_token(identifier.syntax(), SyntaxKind::Word))
}

fn block(node: &SyntaxNode) -> Option<SyntaxNode> {
    node.children()
        .find(|child| child.kind() == SyntaxKind::Block)
}

fn field<N: AstNode>(node: &SyntaxNode, which: Keyword) -> Option<N> {
    block(node)?
        .children()
        .filter_map(N::cast)
        .find(|f| keyword(f.syntax()) == Some(which))
}

fn string_value(text: &str) -> String {
    let inner = text.strip_prefix('"').unwrap_or(text);
    let inner = inner.strip_suffix('"').unwrap_or(inner);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(escaped @ ('"' | '\\')) => out.push(escaped),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn date_value(text: &str) -> Date {
    fn part<T: std::str::FromStr + Default>(text: &str, range: std::ops::Range<usize>) -> T {
        text.get(range)
            .and_then(|s| s.parse().ok())
            .unwrap_or_default()
    }
    Date {
        year: part(text, 0..4),
        month: part(text, 5..7),
        day: part(text, 8..10),
    }
}

#[cfg(test)]
fn markdown_value(text: &str) -> String {
    MarkdownBody::new(text, 0.into()).value
}
