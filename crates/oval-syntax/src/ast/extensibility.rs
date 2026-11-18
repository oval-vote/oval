use crate::Keyword;
use crate::ast::{AstNode, ast_node, support};

ast_node! {
    /// An extensibility written before a ballot, section, or contest, `extensible` or
    /// `exhaustive`.
    ///
    /// It says whether a voter may write in a choice not printed on the ballot. A construct with no
    /// extensibility of its own takes the one nearest above it.
    ///
    /// ```
    /// let text = "extensible ballot @city \"City\"\n  eligibility \"E\"\n";
    /// let result = oval_syntax::parse(text)?;
    /// let ballot = result.document().ballots().next().expect("the document has a ballot");
    /// let extensibility = ballot.extensibility().expect("the ballot has an extensibility");
    /// assert!(extensibility.is_extensible());
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Extensibility
}

impl Extensibility {
    /// Returns `true` if the extensibility is `extensible`, and `false` if it is `exhaustive`.
    pub fn is_extensible(&self) -> bool {
        support::keyword(self.syntax()) == Some(Keyword::Extensible)
    }
}

#[cfg(test)]
mod tests {
    use super::Extensibility;
    use crate::SyntaxKind;
    use crate::ast::AstNode;

    fn extensibility(word: &str) -> Extensibility {
        crate::parse(&format!("{word} ballot @a \"A\"\n  eligibility \"E\"\n"))
            .unwrap()
            .document()
            .ballots()
            .next()
            .and_then(|ballot| ballot.extensibility())
            .expect("the ballot has an extensibility")
    }

    #[test]
    fn extensible_is_extensible() {
        assert!(extensibility("extensible").is_extensible());
    }

    #[test]
    fn exhaustive_is_not_extensible() {
        assert!(!extensibility("exhaustive").is_extensible());
    }

    #[test]
    fn only_an_extensibility_node_is_an_extensibility() {
        assert!(Extensibility::can_cast(SyntaxKind::Extensibility));
        assert!(!Extensibility::can_cast(SyntaxKind::Requirement));
    }
}
