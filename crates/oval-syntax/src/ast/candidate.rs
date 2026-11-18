use crate::Keyword;
use crate::ast::{
    AstNode, Identifier, MarkdownField, PartyRef, StringField, StringLiteral, ast_node, support,
};

ast_node! {
    /// A candidate declared at the top level, such as `candidate @nreyes "Nadia Reyes"`, with the
    /// fields in the block below it.
    ///
    /// ```
    /// let text = "candidate @nreyes \"Nadia Reyes\"\n  party @dem\n";
    /// let result = oval_syntax::parse(text)?;
    /// let candidate = result.document().candidates().next().expect("a candidate");
    /// let name = candidate.name().expect("the candidate has a name");
    /// assert_eq!(name.value(), "Nadia Reyes");
    /// assert!(candidate.party().is_some_and(|party| !party.is_declaration()));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Candidate
}

impl Candidate {
    /// Returns the candidate's identifier, or `None` if the line has none.
    pub fn identifier(&self) -> Option<Identifier> {
        support::child(self.syntax())
    }

    /// Returns the candidate's name, or `None` if the line ends without one.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }

    /// Returns the candidate's `designation` field, or `None` if it has none.
    pub fn designation(&self) -> Option<StringField> {
        support::block_field(self.syntax(), Keyword::Designation)
    }

    /// Returns the candidate's `party` field, or `None` if it has none.
    pub fn party(&self) -> Option<PartyRef> {
        support::block_children(self.syntax()).next()
    }

    /// Returns the candidate's `note` field, or `None` if it has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::block_field(self.syntax(), Keyword::Note)
    }
}

#[cfg(test)]
mod tests {
    use super::Candidate;
    use crate::ast::AstNode;
    use crate::{SyntaxKind, parse};

    fn candidate(text: &str) -> Candidate {
        parse(text)
            .unwrap()
            .document()
            .candidates()
            .next()
            .expect("the document has a candidate")
    }

    #[test]
    fn a_candidate_has_its_identifier_name_and_fields() {
        let candidate = candidate(
            "candidate @nreyes \"Nadia Reyes\"\n  designation \"Senator\"\n  party @dem\n  note:\n    Appointed.\n",
        );
        let name = candidate.identifier().and_then(|id| id.name());
        assert_eq!(
            name.map(|name| name.text().to_owned()).as_deref(),
            Some("nreyes")
        );
        let designation = candidate.designation().and_then(|field| field.value());
        assert_eq!(
            designation
                .map(|value| value.value().into_owned())
                .as_deref(),
            Some("Senator")
        );
        assert!(candidate.party().is_some());
        let note = candidate.note().and_then(|field| field.body());
        assert_eq!(note.map(|body| body.value()).as_deref(), Some("Appointed."));
    }

    #[test]
    fn a_candidate_without_a_block_has_no_fields() {
        let candidate = candidate("candidate @c \"C\"\n");
        assert_eq!(candidate.designation(), None);
        assert_eq!(candidate.party(), None);
        assert_eq!(candidate.note(), None);
    }

    #[test]
    fn a_repeated_party_field_is_not_the_party() {
        let candidate = candidate("candidate @c \"C\"\n  party @dem\n  party @rep\n");
        let party = candidate.party().and_then(|party| party.identifier());
        let name = party.and_then(|id| id.name());
        assert_eq!(
            name.map(|name| name.text().to_owned()).as_deref(),
            Some("dem")
        );
    }

    #[test]
    fn only_a_candidate_node_is_a_candidate() {
        assert!(Candidate::can_cast(SyntaxKind::Candidate));
        assert!(!Candidate::can_cast(SyntaxKind::Party));
    }
}
