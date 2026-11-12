use crate::Keyword;
use crate::ast::{
    AstNode, ContestKind, Extensibility, Identifier, MarkdownField, Requirement, StringField,
    StringLiteral, ast_node, support,
};

ast_node! {
    /// A contest, such as `contest @mayor "Mayor" single` and the block below it, in which voters
    /// choose among campaigns for an office.
    ///
    /// ```
    /// use oval_syntax::ast::{BallotItem, Method};
    ///
    /// let text = r#"ballot @city "City"
    ///   contest @council "City Council" multiple 3 seats 4
    ///     subtitle "Vote for up to three"
    ///     campaign @rosa
    /// "#;
    /// let result = oval_syntax::parse(text)?;
    /// let ballot = result.document().ballots().next().expect("the document has a ballot");
    /// let Some(BallotItem::Contest(contest)) = ballot.items().next() else {
    ///     panic!("the ballot's first item is a contest");
    /// };
    /// let kind = contest.kind().expect("the contest has a kind");
    /// assert_eq!(kind.method(), Method::Multiple);
    /// assert_eq!(kind.votes().and_then(|votes| votes.value()), Some(3));
    /// assert_eq!(kind.seats().and_then(|seats| seats.value()), Some(4));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Contest
}

impl Contest {
    /// Returns the requirement written before `contest`, or `None` if there is none.
    pub fn requirement(&self) -> Option<Requirement> {
        support::child(self.syntax())
    }

    /// Returns the extensibility written before `contest`, or `None` if there is none.
    pub fn extensibility(&self) -> Option<Extensibility> {
        support::child(self.syntax())
    }

    /// Returns the contest's identifier, or `None` if the line has none.
    pub fn identifier(&self) -> Option<Identifier> {
        support::child(self.syntax())
    }

    /// Returns the contest's name, or `None` if the line ends without one.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns the contest's kind and its modifiers, written after its name, or `None` if the line
    /// ends without a kind.
    pub fn kind(&self) -> Option<ContestKind> {
        support::child(self.syntax())
    }

    /// Returns the contest's `subtitle` field, or `None` if it has none.
    pub fn subtitle(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Subtitle)
    }

    /// Returns the contest's `tabulation` field, or `None` if it has none.
    pub fn tabulation(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Tabulation)
    }

    /// Returns the contest's `note` field, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }
}

#[cfg(test)]
pub(super) fn first_contest(lines: &str) -> Contest {
    crate::parse(&format!("ballot @a \"A\"\n{lines}"))
        .unwrap()
        .document()
        .ballots()
        .next()
        .and_then(|ballot| ballot.items().next())
        .and_then(|item| match item {
            crate::ast::BallotItem::Contest(contest) => Some(contest),
            _ => None,
        })
        .expect("the ballot's first item is a contest")
}

#[cfg(test)]
mod tests {
    use super::{Contest, first_contest};
    use crate::SyntaxKind;
    use crate::ast::{AstNode, Method};

    fn string(field: Option<crate::ast::StringField>) -> Option<String> {
        field
            .and_then(|field| field.value())
            .map(|value| value.value().into_owned())
    }

    #[test]
    fn a_contest_has_its_line_and_fields() {
        let contest = first_contest(
            "  required extensible contest @mayor \"Mayor\" ranked max 5\n    subtitle \"Rank up to five\"\n    tabulation \"Instant runoff\"\n    note:\n      N.\n    campaign @a\n",
        );
        assert!(contest.requirement().is_some_and(|r| r.is_required()));
        assert!(contest.extensibility().is_some_and(|e| e.is_extensible()));
        let id = contest.identifier().and_then(|id| id.name());
        assert_eq!(
            id.map(|name| name.text().to_owned()).as_deref(),
            Some("mayor")
        );
        assert_eq!(
            contest
                .name()
                .map(|name| name.value().into_owned())
                .as_deref(),
            Some("Mayor")
        );
        assert_eq!(
            contest.kind().map(|kind| kind.method()),
            Some(Method::Ranked)
        );
        assert_eq!(
            string(contest.subtitle()).as_deref(),
            Some("Rank up to five")
        );
        assert_eq!(
            string(contest.tabulation()).as_deref(),
            Some("Instant runoff")
        );
        assert!(contest.note().is_some());
    }

    #[test]
    fn a_contest_without_a_kind_has_none() {
        let contest = first_contest("  contest @m \"M\"\n    campaign @a\n");
        assert_eq!(contest.kind(), None);
        assert_eq!(contest.subtitle(), None);
    }

    #[test]
    fn only_a_contest_node_is_a_contest() {
        assert!(Contest::can_cast(SyntaxKind::Contest));
        assert!(!Contest::can_cast(SyntaxKind::ContestKind));
    }
}
