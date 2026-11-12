use crate::Keyword;
use crate::ast::{AstNode, MarkdownField, NoStance, Oppose, Stance, Support, ast_node, support};

ast_node! {
    /// A contest's recommendation, `recommend` and the block below it.
    ///
    /// A recommendation states its position in one of two ways. It either has `no-position` or
    /// `abstain` on its line, or its block has `support` and `oppose` lines. A `support` or
    /// `oppose` line below `no-position` or `abstain` is set aside by the parser, so it is not
    /// among the stances.
    ///
    /// ```
    /// let text = r#"ballot @city "City"
    ///   contest @mayor "Mayor" ranked
    ///     campaign @aokafor
    ///     campaign @bsantos
    ///     campaign @cfox
    ///     recommend
    ///       support @aokafor
    ///       support @bsantos
    ///       oppose @cfox
    ///       justification:
    ///         Okafor first.
    /// "#;
    /// let result = oval_syntax::parse(text)?;
    /// let ballot = result.document().ballots().next().expect("the document has a ballot");
    /// let Some(oval_syntax::ast::BallotItem::Contest(contest)) = ballot.items().next() else {
    ///     panic!("the ballot's first item is a contest");
    /// };
    /// let recommendation = contest.recommendations().next().expect("a recommendation");
    /// assert_eq!(recommendation.supports().count(), 2);
    /// assert_eq!(recommendation.opposes().count(), 1);
    /// assert!(recommendation.no_stance().is_none());
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    ContestRecommend
}

impl ContestRecommend {
    /// Returns `no-position` or `abstain` from the `recommend` line, or `None` if the line has
    /// neither.
    pub fn no_stance(&self) -> Option<NoStance> {
        support::child(self.syntax())
    }

    /// Returns the `support` and `oppose` lines, in the order written.
    pub fn stances(&self) -> impl Iterator<Item = Stance> + use<> {
        support::block_children(self.syntax())
    }

    /// Returns the `support` lines, the tiers from most to least preferred.
    pub fn supports(&self) -> impl Iterator<Item = Support> + use<> {
        support::block_children(self.syntax())
    }

    /// Returns the `oppose` lines, in the order written.
    ///
    /// A valid recommendation has at most one. The parser accepts more, so that validation can
    /// report the repetition.
    pub fn opposes(&self) -> impl Iterator<Item = Oppose> + use<> {
        support::block_children(self.syntax())
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
pub(super) fn first_recommendation(lines: &str) -> ContestRecommend {
    super::contest::first_contest(&format!("  contest @c \"C\" single\n{lines}"))
        .recommendations()
        .next()
        .expect("the contest has a recommendation")
}

#[cfg(test)]
mod tests {
    use super::{ContestRecommend, first_recommendation};
    use crate::SyntaxKind;
    use crate::ast::contest::first_contest;
    use crate::ast::{AstNode, Stance};

    #[test]
    fn a_recommendation_has_its_stances_and_fields() {
        let recommendation = first_recommendation(
            "    recommend\n      justification:\n        J.\n      support @a\n      oppose @c\n      support @b\n      note:\n        N.\n",
        );
        assert_eq!(recommendation.no_stance(), None);
        let stances: Vec<bool> = recommendation
            .stances()
            .map(|stance| matches!(stance, Stance::Support(_)))
            .collect();
        assert_eq!(stances, [true, false, true]);
        assert_eq!(recommendation.supports().count(), 2);
        assert_eq!(recommendation.opposes().count(), 1);
        assert!(recommendation.justification().is_some());
        assert!(recommendation.note().is_some());
    }

    #[test]
    fn a_recommendation_without_a_stance_has_its_no_stance() {
        let recommendation =
            first_recommendation("    recommend abstain\n      justification:\n        J.\n");
        assert!(recommendation.no_stance().is_some_and(|n| n.is_abstain()));
        assert_eq!(recommendation.stances().count(), 0);
        assert!(recommendation.justification().is_some());
    }

    #[test]
    fn a_stance_below_no_stance_is_set_aside() {
        let recommendation = first_recommendation("    recommend abstain\n      support @a\n");
        assert_eq!(recommendation.stances().count(), 0);
    }

    #[test]
    fn a_contest_has_every_recommendation() {
        let contest = first_contest(
            "  contest @c \"C\" single\n    campaign @a\n    recommend no-position\n    recommend\n      support @a\n",
        );
        assert_eq!(contest.recommendations().count(), 2);
    }

    #[test]
    fn only_a_contest_recommend_node_is_a_contest_recommend() {
        assert!(ContestRecommend::can_cast(SyntaxKind::ContestRecommend));
        assert!(!ContestRecommend::can_cast(SyntaxKind::MeasureRecommend));
    }
}
