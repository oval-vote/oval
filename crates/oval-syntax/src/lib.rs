//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.

pub mod ast;
pub mod lexer;
pub mod parser;
mod syntax_kind;

pub use rowan::{TextRange, TextSize, WalkEvent};
pub use syntax_kind::{Keyword, SyntaxKind};

/// The major version of the Oval language this crate reads, as documents declare it with
/// `oval 1`.
pub const SUPPORTED_LANGUAGE_MAJOR: u32 = 1;

/// The Oval language, for use with [`rowan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OvalLanguage {}

impl rowan::Language for OvalLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        raw.into()
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

/// A node in an Oval syntax tree.
pub type SyntaxNode = rowan::SyntaxNode<OvalLanguage>;

/// A token in an Oval syntax tree.
pub type SyntaxToken = rowan::SyntaxToken<OvalLanguage>;

/// Either a node or a token.
pub type SyntaxElement = rowan::SyntaxElement<OvalLanguage>;
