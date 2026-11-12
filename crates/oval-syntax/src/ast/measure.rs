use crate::Keyword;
use crate::ast::{
    AstNode, Identifier, MarkdownField, MeasureRecommend, Requirement, StringField,
    StringListField, StringLiteral, ast_node, support,
};

ast_node! {
    /// A ballot measure, such as `measure @prop1 "Proposition 1"` and the block below it, which
    /// voters answer yes or no.
    ///
    /// A measure may have a requirement but takes no extensibility, since there is nothing to
    /// write in.
    ///
    /// ```
    /// use oval_syntax::ast::{BallotItem, Position};
    ///
    /// let text = r#"ballot @state "State"
    ///   measure @prop1 "Proposition 1"
    ///     subtitle "Affordable Housing Bond"
    ///     supporters "Franklin Housing Coalition"
    ///     opponents none
    ///     recommend yes
    /// "#;
    /// let result = oval_syntax::parse(text)?;
    /// let ballot = result.document().ballots().next().expect("the document has a ballot");
    /// let Some(BallotItem::Measure(measure)) = ballot.items().next() else {
    ///     panic!("the ballot's first item is a measure");
    /// };
    /// assert!(measure.opponents().is_some_and(|opponents| opponents.is_none_value()));
    /// let recommendation = measure.recommendations().next().expect("a recommendation");
    /// let position = recommendation.position().expect("the recommendation has a position");
    /// assert_eq!(position.value(), Position::Yes);
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Measure
}

impl Measure {
    /// Returns the requirement written before `measure`, or `None` if there is none.
    pub fn requirement(&self) -> Option<Requirement> {
        support::child(self.syntax())
    }

    /// Returns the measure's identifier, or `None` if the line has none.
    pub fn identifier(&self) -> Option<Identifier> {
        support::child(self.syntax())
    }

    /// Returns the measure's name, or `None` if the line ends without one.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns the measure's `subtitle` field, or `None` if it has none.
    pub fn subtitle(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Subtitle)
    }

    /// Returns the `description` field, the measure's official ballot text, or `None` if it has
    /// none.
    pub fn description(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Description)
    }

    /// Returns the `if_yes` field, the official statement of what a yes vote does, or `None` if it
    /// has none.
    pub fn if_yes(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::IfYes)
    }

    /// Returns the `if_no` field, the official statement of what a no vote does, or `None` if it
    /// has none.
    pub fn if_no(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::IfNo)
    }

    /// Returns the `placement` field, how the measure came to be on the ballot, or `None` if it
    /// has none.
    pub fn placement(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Placement)
    }

    /// Returns the `tabulation` field, how the measure is decided, or `None` if it has none.
    pub fn tabulation(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Tabulation)
    }

    /// Returns the `fiscal_impact` field, or `None` if it has none.
    pub fn fiscal_impact(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::FiscalImpact)
    }

    /// Returns the `supporters` field, or `None` if it has none.
    ///
    /// A measure without the field has supporters no one has looked for. A field written as
    /// `none` has supporters known to be none.
    pub fn supporters(&self) -> Option<StringListField> {
        support::block_field(self.syntax(), Keyword::Supporters)
    }

    /// Returns the `opponents` field, or `None` if it has none.
    pub fn opponents(&self) -> Option<StringListField> {
        support::block_field(self.syntax(), Keyword::Opponents)
    }

    /// Returns the measure's `note` field, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }

    /// Returns the measure's recommendations, in the order written.
    ///
    /// A valid measure has exactly one. The parser accepts any number, so that validation can
    /// report a measure with none or with more than one.
    pub fn recommendations(&self) -> impl Iterator<Item = MeasureRecommend> + use<> {
        support::block_children(self.syntax())
    }
}

#[cfg(test)]
pub(super) fn first_measure(lines: &str) -> Measure {
    crate::parse(&format!("ballot @a \"A\"\n{lines}"))
        .unwrap()
        .document()
        .ballots()
        .next()
        .and_then(|ballot| ballot.items().next())
        .and_then(|item| match item {
            crate::ast::BallotItem::Measure(measure) => Some(measure),
            _ => None,
        })
        .expect("the ballot's first item is a measure")
}

#[cfg(test)]
mod tests {
    use super::{Measure, first_measure};
    use crate::SyntaxKind;
    use crate::ast::AstNode;

    fn string(field: Option<crate::ast::StringField>) -> Option<String> {
        field
            .and_then(|field| field.value())
            .map(|value| value.value().into_owned())
    }

    fn body(field: Option<crate::ast::MarkdownField>) -> Option<String> {
        field
            .and_then(|field| field.body())
            .map(|body| body.value())
    }

    #[test]
    fn a_measure_has_its_line_and_every_field() {
        let measure = first_measure(
            "  required measure @prop1 \"Proposition 1\"\n    subtitle \"Bond\"\n    description:\n      D.\n    if_yes:\n      Y.\n    if_no:\n      N.\n    placement \"Referral\"\n    tabulation \"Majority\"\n    fiscal_impact \"F\"\n    supporters \"A\", \"B\"\n    opponents none\n    note:\n      Note.\n    recommend yes\n",
        );
        assert!(measure.requirement().is_some_and(|r| r.is_required()));
        let id = measure.identifier().and_then(|id| id.name());
        assert_eq!(
            id.map(|name| name.text().to_owned()).as_deref(),
            Some("prop1")
        );
        let name = measure.name().map(|name| name.value().into_owned());
        assert_eq!(name.as_deref(), Some("Proposition 1"));
        assert_eq!(string(measure.subtitle()).as_deref(), Some("Bond"));
        assert_eq!(body(measure.description()).as_deref(), Some("D."));
        assert_eq!(body(measure.if_yes()).as_deref(), Some("Y."));
        assert_eq!(body(measure.if_no()).as_deref(), Some("N."));
        assert_eq!(string(measure.placement()).as_deref(), Some("Referral"));
        assert_eq!(string(measure.tabulation()).as_deref(), Some("Majority"));
        assert_eq!(string(measure.fiscal_impact()).as_deref(), Some("F"));
        assert_eq!(
            measure.supporters().map(|field| field.strings().count()),
            Some(2)
        );
        assert!(
            measure
                .opponents()
                .is_some_and(|field| field.is_none_value())
        );
        assert_eq!(body(measure.note()).as_deref(), Some("Note."));
        assert_eq!(measure.recommendations().count(), 1);
    }

    #[test]
    fn a_measure_without_fields_has_none() {
        let measure = first_measure("  measure @p \"P\"\n    recommend yes\n");
        assert_eq!(measure.requirement(), None);
        assert_eq!(measure.subtitle(), None);
        assert_eq!(measure.description(), None);
        assert_eq!(measure.supporters(), None);
        assert_eq!(measure.opponents(), None);
    }

    #[test]
    fn a_measure_has_every_recommendation() {
        let measure = first_measure("  measure @p \"P\"\n    recommend yes\n    recommend no\n");
        assert_eq!(measure.recommendations().count(), 2);
    }

    #[test]
    fn only_a_measure_node_is_a_measure() {
        assert!(Measure::can_cast(SyntaxKind::Measure));
        assert!(!Measure::can_cast(SyntaxKind::Contest));
    }
}
