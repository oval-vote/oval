use crate::ast::{
    AstNode, Contest, Extensibility, MarkdownField, Measure, Requirement, StringLiteral, ast_node,
    support,
};
use crate::{Keyword, SyntaxKind, SyntaxNode};

ast_node! {
    /// A section of a ballot, such as `section "City Offices"`, which groups contests and
    /// measures under a heading.
    ///
    /// A section has a title but no identifier, and its items are in ballot order.
    ///
    /// ```
    /// use oval_syntax::ast::{BallotItem, SectionItem};
    ///
    /// let text = r#"ballot @city "City"
    ///   section "Offices"
    ///     contest @mayor "Mayor" single
    ///       campaign @rosa
    /// "#;
    /// let result = oval_syntax::parse(text)?;
    /// let ballot = result.document().ballots().next().expect("the document has a ballot");
    /// let Some(BallotItem::Section(section)) = ballot.items().next() else {
    ///     panic!("the ballot's first item is a section");
    /// };
    /// let title = section.title().expect("the section has a title");
    /// assert_eq!(title.value(), "Offices");
    /// assert!(matches!(section.items().next(), Some(SectionItem::Contest(_))));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Section
}

impl Section {
    /// Returns the requirement written before `section`, or `None` if there is none.
    pub fn requirement(&self) -> Option<Requirement> {
        support::child(self.syntax())
    }

    /// Returns the extensibility written before `section`, or `None` if there is none.
    pub fn extensibility(&self) -> Option<Extensibility> {
        support::child(self.syntax())
    }

    /// Returns the section's title, or `None` if the line ends without one.
    pub fn title(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns the section's `note` field, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }

    /// Returns the section's items, its contests and measures, in ballot order.
    pub fn items(&self) -> impl Iterator<Item = SectionItem> + use<> {
        support::block_children(self.syntax())
    }
}

/// One of a section's items, which are listed in ballot order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectionItem {
    /// A contest in the section.
    Contest(Contest),
    /// A measure in the section.
    Measure(Measure),
}

impl AstNode for SectionItem {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::Contest | SyntaxKind::Measure)
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Contest => Contest::cast(node).map(Self::Contest),
            SyntaxKind::Measure => Measure::cast(node).map(Self::Measure),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Contest(contest) => contest.syntax(),
            Self::Measure(measure) => measure.syntax(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Section, SectionItem};
    use crate::ast::{AstNode, BallotItem};
    use crate::{SyntaxKind, parse};

    fn section(lines: &str) -> Section {
        parse(&format!("ballot @a \"A\"\n{lines}"))
            .unwrap()
            .document()
            .ballots()
            .next()
            .and_then(|ballot| ballot.items().next())
            .and_then(|item| match item {
                BallotItem::Section(section) => Some(section),
                _ => None,
            })
            .expect("the ballot's first item is a section")
    }

    #[test]
    fn a_section_has_its_line_note_and_items() {
        let section = section(
            "  optional exhaustive section \"City Offices\"\n    note:\n      N.\n    contest @c \"C\" single\n      campaign @x\n    measure @m \"M\"\n      recommend yes\n",
        );
        assert!(section.requirement().is_some_and(|r| !r.is_required()));
        assert!(section.extensibility().is_some_and(|e| !e.is_extensible()));
        let title = section.title().map(|title| title.value().into_owned());
        assert_eq!(title.as_deref(), Some("City Offices"));
        assert!(section.note().is_some());
        let kinds: Vec<_> = section.items().map(|item| item.syntax().kind()).collect();
        assert_eq!(kinds, [SyntaxKind::Contest, SyntaxKind::Measure]);
    }

    #[test]
    fn a_section_without_modifiers_or_a_title_has_none() {
        let section = section("  section\n    note:\n      N.\n");
        assert_eq!(section.requirement(), None);
        assert_eq!(section.extensibility(), None);
        assert_eq!(section.title(), None);
        assert_eq!(section.items().count(), 0);
    }

    #[test]
    fn a_section_item_is_a_contest_or_measure() {
        assert!(SectionItem::can_cast(SyntaxKind::Contest));
        assert!(SectionItem::can_cast(SyntaxKind::Measure));
        assert!(!SectionItem::can_cast(SyntaxKind::Section));
        assert!(!SectionItem::can_cast(SyntaxKind::Ballot));
        assert!(!Section::can_cast(SyntaxKind::Ballot));
    }
}
