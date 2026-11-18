use rowan::NodeOrToken;

use super::{AstNode, ast_node};
use crate::{SyntaxKind, SyntaxToken};

ast_node! {
    /// An identifier, such as `@dem`, which names a party, candidate, ballot, contest, or measure.
    Identifier
}

impl Identifier {
    /// Returns the word after the `@`, such as `dem` in `@dem`, or `None` if the `@` has no word
    /// after it.
    ///
    /// The word is returned as a token, so its range is at hand along with its text.
    ///
    /// ```
    /// use oval_syntax::ast::{AstNode, Identifier};
    ///
    /// let result = oval_syntax::parse("party @dem \"Democratic\"\n")?;
    /// let identifier = result
    ///     .syntax()
    ///     .descendants()
    ///     .find_map(Identifier::cast)
    ///     .expect("the party has an identifier");
    /// assert_eq!(identifier.name().map(|name| name.text().to_owned()).as_deref(), Some("dem"));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax()
            .children_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .find(|token| token.kind() == SyntaxKind::Word)
    }
}

#[cfg(test)]
mod tests {
    use super::Identifier;
    use crate::ast::AstNode;
    use crate::{SyntaxKind, parse};

    fn identifier(text: &str) -> Identifier {
        parse(text)
            .unwrap()
            .syntax()
            .descendants()
            .find_map(Identifier::cast)
            .expect("the text has an identifier")
    }

    fn name(text: &str) -> Option<String> {
        identifier(text).name().map(|name| name.text().to_owned())
    }

    #[test]
    fn an_identifier_is_named_by_the_word_after_its_at_sign() {
        assert_eq!(name("party @dem \"Democratic\"\n").as_deref(), Some("dem"));
        assert_eq!(
            name("candidate @dana-lombardi \"Dana Lombardi\"\n").as_deref(),
            Some("dana-lombardi")
        );
    }

    #[test]
    fn a_name_keeps_its_range() {
        let name = identifier("party @dem \"Democratic\"\n")
            .name()
            .expect("the identifier has a name");
        assert_eq!(u32::from(name.text_range().start()), 7);
        assert_eq!(u32::from(name.text_range().end()), 10);
    }

    #[test]
    fn an_at_sign_without_a_word_has_no_name() {
        assert_eq!(name("party @ \"Democratic\"\n"), None);
        assert_eq!(name("party @\n"), None);
    }

    #[test]
    fn only_an_identifier_node_is_an_identifier() {
        assert!(Identifier::can_cast(SyntaxKind::Identifier));
        let party = parse("party @dem \"Democratic\"\n")
            .unwrap()
            .syntax()
            .first_child()
            .expect("the document has a party");
        assert_eq!(party.kind(), SyntaxKind::Party);
        assert_eq!(Identifier::cast(party), None);
    }
}
