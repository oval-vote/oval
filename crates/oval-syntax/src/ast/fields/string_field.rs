use crate::Keyword;
use crate::ast::{AstNode, StringLiteral, ast_node, support};

ast_node! {
    /// A field whose value is a string, such as `guide "Franklin Voter Guide"`.
    ///
    /// ```
    /// let result = oval_syntax::parse("guide \"Franklin Voter Guide\"\n")?;
    /// let guide = result.document().guide().expect("the document has a guide field");
    /// let value = guide.value().expect("the field has a value");
    /// assert_eq!(value.value(), "Franklin Voter Guide");
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    StringField
}

impl StringField {
    /// Returns the field's keyword, such as [`Keyword::Guide`].
    ///
    /// # Panics
    ///
    /// Never, since the parser builds a field only once it has read the field's keyword.
    pub fn keyword(&self) -> Keyword {
        support::keyword(self.syntax()).expect("a field begins with its keyword")
    }

    /// Returns the field's string, or `None` if the line ends without one or has another kind of
    /// token in its place.
    pub fn value(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }
}

#[cfg(test)]
mod tests {
    use super::StringField;
    use crate::ast::AstNode;
    use crate::ast::fields::first;
    use crate::{Keyword, SyntaxKind, parse};

    fn value(text: &str) -> Option<String> {
        first::<StringField>(text)
            .value()
            .map(|string| string.value().into_owned())
    }

    #[test]
    fn a_string_field_has_its_keyword_and_value() {
        let field = first::<StringField>("guide \"Franklin Voter Guide\"\n");
        assert_eq!(field.keyword(), Keyword::Guide);
        assert_eq!(
            value("guide \"Franklin Voter Guide\"\n").as_deref(),
            Some("Franklin Voter Guide")
        );
        assert_eq!(
            first::<StringField>("election \"E\"\n").keyword(),
            Keyword::Election
        );
    }

    #[test]
    fn a_field_without_a_value_has_none() {
        assert_eq!(value("guide\n"), None);
    }

    #[test]
    fn a_value_of_another_kind_is_not_a_value() {
        assert_eq!(value("guide 5\n"), None);
    }

    #[test]
    fn a_skipped_token_after_the_value_does_not_change_it() {
        assert_eq!(value("guide \"G\" \"H\"\n").as_deref(), Some("G"));
    }

    #[test]
    fn only_a_string_field_node_is_a_string_field() {
        assert!(StringField::can_cast(SyntaxKind::StringField));
        let date = parse("date 2026-11-03\n")
            .unwrap()
            .syntax()
            .first_child()
            .expect("the document has a field");
        assert_eq!(StringField::cast(date), None);
    }
}
