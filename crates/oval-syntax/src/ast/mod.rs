//! Typed views over the syntax tree.

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

/// A typed wrapper around a [`SyntaxNode`] of one particular kind.
pub trait AstNode: Sized {
    /// Whether nodes of `kind` can be viewed as this type.
    fn can_cast(kind: SyntaxKind) -> bool;

    /// Wraps `node` if it has the right kind.
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// The underlying node.
    fn syntax(&self) -> &SyntaxNode;

    /// The source range the node covers.
    fn range(&self) -> TextRange {
        self.syntax().text_range()
    }

    /// The range of the node's first line, up to its line terminator.
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
    /// An identifier: `@` followed by a word.
    Identifier
}

impl Identifier {
    /// The name after the `@`.
    pub fn name(&self) -> String {
        token(&self.0, SyntaxKind::Word).map_or_else(String::new, |word| word.text().to_owned())
    }
}

/// A calendar date as written, with no check that it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date {
    /// Four-digit year.
    pub year: u16,
    /// Month, 1 through 12 if valid.
    pub month: u8,
    /// Day of month, 1 through 31 if valid.
    pub day: u8,
}

ast_node! {
    /// A field whose value is a string.
    StringField
}

ast_node! {
    /// A field whose value is a date.
    DateField
}

ast_node! {
    /// A field whose value is a URL.
    UrlField
}

ast_node! {
    /// A field whose value is a Markdown body.
    MarkdownField
}

impl StringField {
    /// Which field this is.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The string with its escapes resolved.
    pub fn value(&self) -> Option<String> {
        token(&self.0, SyntaxKind::StringLiteral).map(|t| string_value(t.text()))
    }
}

impl DateField {
    /// Which field this is.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The date as written.
    pub fn value(&self) -> Option<Date> {
        token(&self.0, SyntaxKind::DateLiteral).map(|t| date_value(t.text()))
    }

    /// The range of the date itself.
    pub fn value_range(&self) -> Option<TextRange> {
        token_range(&self.0, SyntaxKind::DateLiteral)
    }
}

impl UrlField {
    /// Which field this is.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The URL text.
    pub fn value(&self) -> Option<String> {
        token(&self.0, SyntaxKind::UrlLiteral).map(|t| t.text().to_owned())
    }

    /// The range of the URL itself.
    pub fn value_range(&self) -> Option<TextRange> {
        token_range(&self.0, SyntaxKind::UrlLiteral)
    }
}

impl MarkdownField {
    /// Which field this is.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The body with its common indentation removed, line terminators normalized to `\n`,
    /// and no trailing terminator.
    pub fn value(&self) -> Option<String> {
        self.body().map(|body| body.value)
    }

    /// The body's value together with the mapping back to its source positions.
    pub fn body(&self) -> Option<MarkdownBody> {
        token(&self.0, SyntaxKind::MarkdownBlock)
            .map(|t| MarkdownBody::new(t.text(), t.text_range().start()))
    }
}

/// The value of a Markdown body and where each of its lines sits in the source.
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

    /// The source offset of a byte offset within the value.
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

    /// The source range of a byte range within the value.
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
