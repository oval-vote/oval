use crate::ast::{
    AstNode, Contest, Extensibility, Identifier, MarkdownField, Measure, Requirement, Section,
    StringField, StringLiteral, ast_node, support,
};
use crate::{Keyword, SyntaxKind, SyntaxNode};

ast_node! {
    /// A ballot, such as `ballot @franklin "State of Franklin"`, with its fields and items in the
    /// block below it.
    ///
    /// A ballot at the top level is a root ballot. A ballot among another ballot's items is a
    /// sub-ballot, for a narrower group of voters who also see everything in the ballot around it.
    ///
    /// ```
    /// use oval_syntax::ast::BallotItem;
    ///
    /// let text = r#"ballot @city "City"
    ///   section "Offices"
    ///     note:
    ///       Offices of the city.
    ///   ballot @ward "Ward 1"
    ///     eligibility "Voters in Ward 1"
    /// "#;
    /// let result = oval_syntax::parse(text)?;
    /// let ballot = result.document().ballots().next().expect("the document has a ballot");
    /// let items: Vec<BallotItem> = ballot.items().collect();
    /// assert!(matches!(items[..], [BallotItem::Section(_), BallotItem::Ballot(_)]));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Ballot
}

impl Ballot {
    /// Returns the requirement written before `ballot`, or `None` if there is none.
    pub fn requirement(&self) -> Option<Requirement> {
        support::child(self.syntax())
    }

    /// Returns the extensibility written before `ballot`, or `None` if there is none.
    pub fn extensibility(&self) -> Option<Extensibility> {
        support::child(self.syntax())
    }

    /// Returns the ballot's identifier, or `None` if the line has none.
    pub fn identifier(&self) -> Option<Identifier> {
        support::child(self.syntax())
    }

    /// Returns the ballot's name, or `None` if the line ends without one.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns the ballot's `eligibility` field, or `None` if it has none.
    pub fn eligibility(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Eligibility)
    }

    /// Returns the ballot's `note` field, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }

    /// Returns the ballot's items, its sections, contests, measures, and sub-ballots, in ballot
    /// order.
    pub fn items(&self) -> impl Iterator<Item = BallotItem> + use<> {
        support::block_children(self.syntax())
    }
}

/// One of a ballot's items, which are listed in ballot order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BallotItem {
    /// A section of the ballot.
    Section(Section),
    /// A contest directly on the ballot.
    Contest(Contest),
    /// A measure directly on the ballot.
    Measure(Measure),
    /// A sub-ballot.
    Ballot(Ballot),
}

impl AstNode for BallotItem {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::Section | SyntaxKind::Contest | SyntaxKind::Measure | SyntaxKind::Ballot
        )
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Section => Section::cast(node).map(Self::Section),
            SyntaxKind::Contest => Contest::cast(node).map(Self::Contest),
            SyntaxKind::Measure => Measure::cast(node).map(Self::Measure),
            SyntaxKind::Ballot => Ballot::cast(node).map(Self::Ballot),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Section(section) => section.syntax(),
            Self::Contest(contest) => contest.syntax(),
            Self::Measure(measure) => measure.syntax(),
            Self::Ballot(ballot) => ballot.syntax(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Ballot, BallotItem};
    use crate::ast::AstNode;
    use crate::{SyntaxKind, parse};

    fn ballot(text: &str) -> Ballot {
        parse(text)
            .unwrap()
            .document()
            .ballots()
            .next()
            .expect("the document has a ballot")
    }

    fn item_kinds(ballot: &Ballot) -> Vec<SyntaxKind> {
        ballot.items().map(|item| item.syntax().kind()).collect()
    }

    #[test]
    fn a_ballot_has_its_line_and_fields() {
        let ballot = ballot(
            "required extensible ballot @city \"City of Ashford\"\n  eligibility \"Voters\"\n  note:\n    N.\n",
        );
        assert!(ballot.requirement().is_some_and(|r| r.is_required()));
        assert!(ballot.extensibility().is_some_and(|e| e.is_extensible()));
        let id = ballot.identifier().and_then(|id| id.name());
        assert_eq!(
            id.map(|name| name.text().to_owned()).as_deref(),
            Some("city")
        );
        let name = ballot.name().map(|name| name.value().into_owned());
        assert_eq!(name.as_deref(), Some("City of Ashford"));
        let eligibility = ballot.eligibility().and_then(|field| field.value());
        assert_eq!(
            eligibility
                .map(|value| value.value().into_owned())
                .as_deref(),
            Some("Voters")
        );
        assert!(ballot.note().is_some());
    }

    #[test]
    fn a_ballot_without_modifiers_has_none() {
        let ballot = ballot("ballot @a \"A\"\n  eligibility \"E\"\n");
        assert_eq!(ballot.requirement(), None);
        assert_eq!(ballot.extensibility(), None);
    }

    #[test]
    fn a_ballot_has_every_kind_of_item_in_order() {
        let ballot = ballot(
            "ballot @a \"A\"\n  eligibility \"E\"\n  section \"S\"\n    note:\n      N.\n  contest @c \"C\" single\n    campaign @x\n  measure @m \"M\"\n    recommend yes\n  ballot @sub \"Sub\"\n    eligibility \"E\"\n",
        );
        assert_eq!(
            item_kinds(&ballot),
            [
                SyntaxKind::Section,
                SyntaxKind::Contest,
                SyntaxKind::Measure,
                SyntaxKind::Ballot
            ]
        );
    }

    #[test]
    fn an_entry_set_aside_is_not_an_item() {
        let ballot = ballot(
            "ballot @a \"A\"\n  extensible measure @m \"M\"\n    recommend yes\n  contest @c \"C\" single\n    campaign @x\n",
        );
        assert_eq!(item_kinds(&ballot), [SyntaxKind::Contest]);
    }

    #[test]
    fn a_ballot_item_is_a_section_contest_measure_or_ballot() {
        for kind in [
            SyntaxKind::Section,
            SyntaxKind::Contest,
            SyntaxKind::Measure,
            SyntaxKind::Ballot,
        ] {
            assert!(BallotItem::can_cast(kind), "{kind:?}");
        }
        assert!(!BallotItem::can_cast(SyntaxKind::Party));
        assert!(!Ballot::can_cast(SyntaxKind::Section));
    }
}
