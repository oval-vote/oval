//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.
//!
//! This crate is the reference implementation of Section 2 of the specification and of the lexical
//! and syntax errors of Section 3. It turns source text into tokens ([`lexer`]), tokens into a
//! tree ([`parser`]), and the tree into typed views ([`ast`]) that the validator, the formatter,
//! and editors read.
//!
//! The tree is lossless. Every byte of the source belongs to exactly one token, including
//! whitespace, comments, and text the lexer could not tokenize, so the text of the root node is
//! the source itself. Errors never stop the parser. It records each one, recovers as the
//! specification prescribes, and still produces a tree that covers every byte.
//!
//! Positions are byte offsets into the UTF-8 source, expressed with [`TextSize`] and
//! [`TextRange`] from [`rowan`], which provides the tree.
//!
//! # Examples
//!
//! ```
//! use oval_syntax::ast::{AstNode, Document};
//! use oval_syntax::parser::parse;
//!
//! let result = parse("oval 1\nguide \"Franklin Voter Guide\"\n").unwrap();
//! assert!(result.errors().is_empty());
//! assert_eq!(result.syntax().text().to_string(), "oval 1\nguide \"Franklin Voter Guide\"\n");
//!
//! let document = Document::cast(result.syntax()).unwrap();
//! assert_eq!(document.version().and_then(|marker| marker.value()), Some(1));
//! ```

pub mod ast;
pub mod lexer;
pub mod parser;
mod syntax_kind;
pub mod text;

pub use rowan::{TextRange, TextSize, WalkEvent};
pub use syntax_kind::{Keyword, SyntaxKind};

/// The major version of the Oval language this crate reads.
///
/// Documents declare their major version in the marker that opens them, `oval 1`. The language
/// has no minor version. Within a major version the specification only adds constructs, so an
/// implementation of a later edition reads every earlier document. A document that names another
/// major version is reported as `unsupported-version` rather than parsed.
pub const SUPPORTED_LANGUAGE_MAJOR: u32 = 1;

/// The Oval language, as [`rowan`] needs it to type the tree.
///
/// It has no values. It only ties [`SyntaxKind`] to the tree types below.
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

/// A node in an Oval syntax tree, which is a document, an entry, a block, or an identifier.
///
/// Nodes own their children and cover the exact span of source text those children do. The typed
/// views in [`ast`] wrap nodes of particular kinds.
pub type SyntaxNode = rowan::SyntaxNode<OvalLanguage>;

/// A token in an Oval syntax tree, which is a leaf with a [`SyntaxKind`] and the text it covers.
pub type SyntaxToken = rowan::SyntaxToken<OvalLanguage>;

/// Either a [`SyntaxNode`] or a [`SyntaxToken`], as a tree walk yields them.
pub type SyntaxElement = rowan::SyntaxElement<OvalLanguage>;
