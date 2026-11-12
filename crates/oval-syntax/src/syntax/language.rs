use crate::SyntaxKind;

/// The Oval language, for use as the type parameter of rowan's syntax tree types.
///
/// `OvalLanguage` has no values. It exists only to tell rowan that the kinds in an Oval syntax tree
/// are [`SyntaxKind`]s. Use the [`SyntaxNode`], [`SyntaxToken`], and [`SyntaxElement`] aliases
/// rather than naming it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OvalLanguage {}

impl rowan::Language for OvalLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::ALL
            .get(usize::from(raw.0))
            .copied()
            .expect("rowan only stores kinds made by kind_to_raw")
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

/// A node in an Oval syntax tree, such as a whole document.
///
/// A node knows its parent and its position in the document, and its text is the text of every
/// token inside it.
pub type SyntaxNode = rowan::SyntaxNode<OvalLanguage>;

/// A token in an Oval syntax tree, which is a leaf of the tree.
///
/// Every token the lexer produces is in the tree, including whitespace, comments, and errors, so
/// the tokens of a tree reproduce its document exactly.
pub type SyntaxToken = rowan::SyntaxToken<OvalLanguage>;

/// Either a [`SyntaxNode`] or a [`SyntaxToken`], such as one child of a node.
pub type SyntaxElement = rowan::SyntaxElement<OvalLanguage>;

#[cfg(test)]
mod tests {
    use super::*;
    use rowan::Language;

    #[test]
    fn every_kind_survives_a_round_trip_through_rowan() {
        for kind in SyntaxKind::ALL {
            let raw = OvalLanguage::kind_to_raw(kind);
            assert_eq!(OvalLanguage::kind_from_raw(raw), kind, "{kind:?}");
        }
    }

    #[test]
    fn a_kind_is_stored_as_its_position() {
        assert_eq!(
            OvalLanguage::kind_to_raw(SyntaxKind::Word),
            rowan::SyntaxKind(0)
        );
        assert_eq!(
            OvalLanguage::kind_to_raw(SyntaxKind::Document),
            rowan::SyntaxKind(17)
        );
    }

    #[test]
    #[should_panic(expected = "rowan only stores kinds made by kind_to_raw")]
    fn a_raw_kind_that_is_not_a_syntax_kind_panics() {
        OvalLanguage::kind_from_raw(rowan::SyntaxKind(u16::MAX));
    }
}
