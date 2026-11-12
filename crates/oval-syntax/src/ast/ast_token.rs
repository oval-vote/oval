use crate::{SyntaxKind, SyntaxToken};

/// A typed view of a [`SyntaxToken`] of one kind, such as a
/// [`StringLiteral`](super::StringLiteral) for a [`String`](SyntaxKind::String) token.
pub trait AstToken: Sized {
    /// Returns `true` if a token of `kind` can be viewed as this type.
    fn can_cast(kind: SyntaxKind) -> bool;

    /// Returns a view of `token`, or `None` if a token of its kind cannot be viewed as this type.
    fn cast(token: SyntaxToken) -> Option<Self>;

    /// Returns the token this view wraps.
    fn syntax(&self) -> &SyntaxToken;

    /// Returns the token's text, exactly as written in the document.
    fn text(&self) -> &str {
        self.syntax().text()
    }
}

macro_rules! ast_token {
    ($(#[$doc:meta])* $name:ident($kind:ident)) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name($crate::SyntaxToken);

        impl $crate::ast::AstToken for $name {
            fn can_cast(kind: $crate::SyntaxKind) -> bool {
                kind == $crate::SyntaxKind::$kind
            }

            fn cast(token: $crate::SyntaxToken) -> Option<Self> {
                Self::can_cast(token.kind()).then(|| Self(token))
            }

            fn syntax(&self) -> &$crate::SyntaxToken {
                &self.0
            }
        }
    };
}

pub(crate) use ast_token;
