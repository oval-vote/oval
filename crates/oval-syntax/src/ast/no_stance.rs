use crate::Keyword;
use crate::ast::{AstNode, ast_node, support};

ast_node! {
    /// A recommendation's statement on its `recommend` line that it takes no stance on the
    /// campaigns, `no-position` or `abstain`.
    NoStance
}

impl NoStance {
    /// Returns `true` if the statement is `no-position`, which means the guide offers no advice.
    pub fn is_no_position(&self) -> bool {
        support::keyword(self.syntax()) == Some(Keyword::NoPosition)
    }

    /// Returns `true` if the statement is `abstain`, which advises leaving the question blank.
    pub fn is_abstain(&self) -> bool {
        support::keyword(self.syntax()) == Some(Keyword::Abstain)
    }
}

#[cfg(test)]
mod tests {
    use super::NoStance;
    use crate::SyntaxKind;
    use crate::ast::AstNode;
    use crate::ast::contest_recommend::first_recommendation;

    fn no_stance(word: &str) -> NoStance {
        first_recommendation(&format!("    recommend {word}\n"))
            .no_stance()
            .expect("the recommendation has no stance")
    }

    #[test]
    fn no_position_is_no_position() {
        let no_stance = no_stance("no-position");
        assert!(no_stance.is_no_position());
        assert!(!no_stance.is_abstain());
    }

    #[test]
    fn abstain_is_abstain() {
        let no_stance = no_stance("abstain");
        assert!(no_stance.is_abstain());
        assert!(!no_stance.is_no_position());
    }

    #[test]
    fn only_a_no_stance_node_is_a_no_stance() {
        assert!(NoStance::can_cast(SyntaxKind::NoStance));
        assert!(!NoStance::can_cast(SyntaxKind::MeasurePosition));
    }
}
