use crate::Keyword;
use crate::ast::{AstNode, MarkdownField, MeasurePosition, ast_node, support};

ast_node! {
    /// A measure's recommendation, such as `recommend yes`, and the fields in the block below it.
    ///
    /// A measure has no campaigns, so its position is always on the `recommend` line, and its
    /// block holds only `justification` and `note`.
    MeasureRecommend
}

impl MeasureRecommend {
    /// Returns the position on the `recommend` line, or `None` if the line ends without one or
    /// has another word in its place.
    pub fn position(&self) -> Option<MeasurePosition> {
        support::child(self.syntax())
    }

    /// Returns the `justification` field, or `None` if there is none.
    pub fn justification(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Justification)
    }

    /// Returns the `note` field, or `None` if there is none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }
}

#[cfg(test)]
mod tests {
    use super::MeasureRecommend;
    use crate::SyntaxKind;
    use crate::ast::AstNode;
    use crate::ast::measure::first_measure;

    fn recommendation(lines: &str) -> MeasureRecommend {
        first_measure(&format!("  measure @p \"P\"\n{lines}"))
            .recommendations()
            .next()
            .expect("the measure has a recommendation")
    }

    #[test]
    fn a_recommendation_has_its_position_and_fields() {
        let recommendation = recommendation(
            "    recommend no\n      justification:\n        J.\n      note:\n        N.\n",
        );
        assert!(recommendation.position().is_some());
        assert!(recommendation.justification().is_some());
        assert!(recommendation.note().is_some());
    }

    #[test]
    fn a_recommendation_without_a_position_has_none() {
        assert_eq!(recommendation("    recommend\n").position(), None);
        assert_eq!(recommendation("    recommend maybe\n").position(), None);
    }

    #[test]
    fn a_recommendation_without_a_block_has_no_fields() {
        let recommendation = recommendation("    recommend yes\n");
        assert_eq!(recommendation.justification(), None);
        assert_eq!(recommendation.note(), None);
    }

    #[test]
    fn only_a_measure_recommend_node_is_a_measure_recommend() {
        assert!(MeasureRecommend::can_cast(SyntaxKind::MeasureRecommend));
        assert!(!MeasureRecommend::can_cast(SyntaxKind::ContestRecommend));
    }
}
