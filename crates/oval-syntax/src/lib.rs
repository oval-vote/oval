//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.

pub mod ast;
pub mod lexer;
pub mod parser;
mod syntax_kind;

use std::fmt;

pub use rowan::{TextRange, TextSize, WalkEvent};
pub use syntax_kind::{Keyword, SyntaxKind};

/// A version of the Oval specification, as written in a document's `oval` marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpecVersion {
    /// The major version. Documents of different major versions are different languages.
    pub major: u32,
    /// The minor version. From major version 1 onward, a minor version only adds constructs.
    pub minor: u32,
}

impl fmt::Display for SpecVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// The version of the Oval specification this crate implements: the major and minor components
/// of the crate's own version.
pub const SPEC_VERSION: SpecVersion = SpecVersion {
    major: decimal(env!("CARGO_PKG_VERSION_MAJOR")),
    minor: decimal(env!("CARGO_PKG_VERSION_MINOR")),
};

#[allow(clippy::cast_lossless)]
const fn decimal(digits: &str) -> u32 {
    let bytes = digits.as_bytes();
    let mut value = 0;
    let mut i = 0;
    while i < bytes.len() {
        assert!(bytes[i].is_ascii_digit(), "a version component is decimal");
        value = value * 10 + (bytes[i] - b'0') as u32;
        i += 1;
    }
    value
}

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
    use super::{SPEC_VERSION, SpecVersion};

    #[test]
    fn spec_version_is_the_crate_version_without_its_patch() {
        let crate_version = env!("CARGO_PKG_VERSION");
        assert!(crate_version.starts_with(&format!("{SPEC_VERSION}.")));
        assert_eq!(SPEC_VERSION.to_string(), "0.1");
    }

    #[test]
    fn spec_version_displays_as_major_minor() {
        assert_eq!(
            SpecVersion {
                major: 12,
                minor: 30
            }
            .to_string(),
            "12.30"
        );
    }

    #[test]
    fn spec_versions_order_by_major_then_minor() {
        let v = |major, minor| SpecVersion { major, minor };
        assert!(v(0, 9) < v(1, 0));
        assert!(v(1, 2) < v(1, 10));
    }
}
