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
pub use document::{Candidate, Document, Party, PartyReference};
pub use measure::{Measure, MeasureRecommend, Position, StringListField};
pub use recommend::{ContestRecommend, Oppose, Support};

use crate::{Keyword, SyntaxKind, SyntaxNode, SyntaxToken};

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

/// An `@name` token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier(SyntaxToken);

impl Identifier {
    /// The name without its leading `@`.
    pub fn name(&self) -> &str {
        &self.0.text()[1..]
    }

    /// The underlying token.
    pub fn syntax(&self) -> &SyntaxToken {
        &self.0
    }

    /// The source range of the token.
    pub fn range(&self) -> TextRange {
        self.0.text_range()
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
}

impl MarkdownField {
    /// Which field this is.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The body with its common indentation removed, line terminators normalized to `\n`,
    /// and no trailing terminator.
    pub fn value(&self) -> Option<String> {
        token(&self.0, SyntaxKind::MarkdownBlock).map(|t| markdown_value(t.text()))
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
    token(node, SyntaxKind::Identifier).map(Identifier)
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

fn markdown_value(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let indentation = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0);
    let mut out = String::with_capacity(text.len());
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if !line.trim().is_empty() {
            out.push_str(&line[indentation..]);
        }
    }
    out
}
