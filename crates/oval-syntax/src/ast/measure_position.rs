use crate::Keyword;
use crate::ast::{AstNode, ast_node, support};

ast_node! {
    /// A measure recommendation's position, the word after `recommend`.
    MeasurePosition
}

/// What a measure's recommendation advises, the value of its [`MeasurePosition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Position {
    /// `yes`, a vote for the measure.
    Yes,
    /// `no`, a vote against the measure.
    No,
    /// `no-position`, no advice.
    NoPosition,
    /// `abstain`, leaving the measure blank.
    Abstain,
}

impl MeasurePosition {
    /// Returns what the position advises.
    ///
    /// # Panics
    ///
    /// Never, since the parser builds a position only from one of its four keywords.
    pub fn value(&self) -> Position {
        match support::keyword(self.syntax()) {
            Some(Keyword::Yes) => Position::Yes,
            Some(Keyword::No) => Position::No,
            Some(Keyword::NoPosition) => Position::NoPosition,
            Some(Keyword::Abstain) => Position::Abstain,
            other => unreachable!("a measure position is one of its keywords, not {other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MeasurePosition, Position};
    use crate::SyntaxKind;
    use crate::ast::AstNode;
    use crate::ast::measure::first_measure;

    fn position(word: &str) -> Position {
        first_measure(&format!("  measure @p \"P\"\n    recommend {word}\n"))
            .recommendations()
            .next()
            .and_then(|recommendation| recommendation.position())
            .expect("the recommendation has a position")
            .value()
    }

    #[test]
    fn each_keyword_is_a_position() {
        assert_eq!(position("yes"), Position::Yes);
        assert_eq!(position("no"), Position::No);
        assert_eq!(position("no-position"), Position::NoPosition);
        assert_eq!(position("abstain"), Position::Abstain);
    }

    #[test]
    fn only_a_measure_position_node_is_a_measure_position() {
        assert!(MeasurePosition::can_cast(SyntaxKind::MeasurePosition));
        assert!(!MeasurePosition::can_cast(SyntaxKind::NoStance));
    }
}
