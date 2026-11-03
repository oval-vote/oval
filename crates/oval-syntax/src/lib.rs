//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.

pub mod lexer;
mod syntax_kind;

pub use syntax_kind::{Keyword, SyntaxKind};

/// The version of the Oval specification this crate implements.
pub const SPEC_VERSION: &str = "0.1";

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

#[cfg(test)]
mod tests {
    use super::SPEC_VERSION;

    #[test]
    fn spec_version_is_a_major_minor_pair() {
        let parts: Vec<&str> = SPEC_VERSION.split('.').collect();
        assert_eq!(parts.len(), 2, "expected MAJOR.MINOR, got {SPEC_VERSION:?}");
        for part in parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "non-numeric component in {SPEC_VERSION:?}"
            );
        }
    }
}
