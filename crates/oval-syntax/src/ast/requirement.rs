use crate::Keyword;
use crate::ast::{AstNode, ast_node, support};

ast_node! {
    /// A requirement written before a ballot, section, contest, or measure, `required` or
    /// `optional`.
    ///
    /// It says whether a ballot that marks anything must answer the questions it covers. A
    /// construct with no requirement of its own takes the one nearest above it.
    ///
    /// ```
    /// let text = "required ballot @city \"City\"\n  eligibility \"E\"\n";
    /// let result = oval_syntax::parse(text)?;
    /// let ballot = result.document().ballots().next().expect("the document has a ballot");
    /// let requirement = ballot.requirement().expect("the ballot has a requirement");
    /// assert!(requirement.is_required());
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Requirement
}

impl Requirement {
    /// Returns `true` if the requirement is `required`, and `false` if it is `optional`.
    pub fn is_required(&self) -> bool {
        support::keyword(self.syntax()) == Some(Keyword::Required)
    }
}

#[cfg(test)]
mod tests {
    use super::Requirement;
    use crate::SyntaxKind;
    use crate::ast::AstNode;

    fn requirement(word: &str) -> Requirement {
        crate::parse(&format!("{word} ballot @a \"A\"\n  eligibility \"E\"\n"))
            .unwrap()
            .document()
            .ballots()
            .next()
            .and_then(|ballot| ballot.requirement())
            .expect("the ballot has a requirement")
    }

    #[test]
    fn required_is_required() {
        assert!(requirement("required").is_required());
    }

    #[test]
    fn optional_is_not_required() {
        assert!(!requirement("optional").is_required());
    }

    #[test]
    fn only_a_requirement_node_is_a_requirement() {
        assert!(Requirement::can_cast(SyntaxKind::Requirement));
        assert!(!Requirement::can_cast(SyntaxKind::Extensibility));
    }
}
