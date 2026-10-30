//! Lossless parser for the Oval Vote format.
//!
//! [`parse`] turns source text into a [`Parse`]: a rowan green tree that
//! round-trips the input byte for byte, plus the diagnostics found while
//! building it. The tree is always complete; unexpected lines become `ERROR`
//! nodes and parsing continues on the next line.
//!
//! The [`ast`] module gives typed access to the tree; [`inline`] parses the
//! inline markup of rationale paragraphs, descriptions, and notes.

pub mod ast;
pub mod diagnostic;
pub mod inline;
pub mod kind;
mod parser;

pub use diagnostic::{has_errors, Diagnostic, Diagnostics, LineIndex, Severity};
pub use kind::{Lang, SyntaxKind};
pub use parser::{parse, Parse};
pub use rowan::{TextRange, TextSize};

pub type SyntaxNode = rowan::SyntaxNode<Lang>;
pub type SyntaxToken = rowan::SyntaxToken<Lang>;
pub type SyntaxElement = rowan::SyntaxElement<Lang>;

/// A piece of text with the source range it came from. Joined paragraphs,
/// field values, and notes are lists of these, one per source line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub range: TextRange,
}

/// Join segments with single spaces, as the spec says for every multi-line
/// construct.
pub fn join_segments(segments: &[Segment]) -> String {
    segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ")
}
