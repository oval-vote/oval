use crate::ast::{AstNode, Identifier, WriteIn};
use crate::{SyntaxKind, SyntaxNode};

/// A choice in a `support` or `oppose` line, a reference to a campaign or a write-in.
///
/// A reference names a campaign by one of its members, a candidate in a candidate campaign or a
/// party in a party campaign.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Choice {
    /// A reference to a campaign, by the identifier of one of its members.
    Reference(Identifier),
    /// A write-in.
    WriteIn(WriteIn),
}

impl AstNode for Choice {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::Identifier | SyntaxKind::WriteIn)
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Identifier => Identifier::cast(node).map(Self::Reference),
            SyntaxKind::WriteIn => WriteIn::cast(node).map(Self::WriteIn),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Reference(identifier) => identifier.syntax(),
            Self::WriteIn(write_in) => write_in.syntax(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Choice;
    use crate::SyntaxKind;
    use crate::ast::AstNode;

    #[test]
    fn a_choice_is_a_reference_or_a_write_in() {
        assert!(Choice::can_cast(SyntaxKind::Identifier));
        assert!(Choice::can_cast(SyntaxKind::WriteIn));
        assert!(!Choice::can_cast(SyntaxKind::Support));
    }
}
