use crate::Keyword;
use crate::ast::{AstNode, DateLiteral, ast_node, support};

ast_node! {
    /// A field whose value is a date, such as `date 2026-11-03`.
    ///
    /// ```
    /// use oval_syntax::ast::{AstNode, DateField};
    ///
    /// let result = oval_syntax::parse("date 2026-11-03\n")?;
    /// let date = result
    ///     .syntax()
    ///     .descendants()
    ///     .find_map(DateField::cast)
    ///     .expect("the document has a date field");
    /// let value = date.value().expect("the field has a value");
    /// assert_eq!((value.year(), value.month(), value.day()), (2026, 11, 3));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    DateField
}

impl DateField {
    /// Returns the field's keyword, such as [`Keyword::Date`].
    ///
    /// # Panics
    ///
    /// Never, since the parser builds a field only once it has read the field's keyword.
    pub fn keyword(&self) -> Keyword {
        support::keyword(self.syntax()).expect("a field begins with its keyword")
    }

    /// Returns the field's date, or `None` if the line ends without one or has another kind of
    /// token in its place.
    pub fn value(&self) -> Option<DateLiteral> {
        support::token(self.syntax())
    }
}

#[cfg(test)]
mod tests {
    use super::DateField;
    use crate::ast::fields::first;
    use crate::ast::{AstNode, AstToken};
    use crate::{Keyword, SyntaxKind};

    #[test]
    fn a_date_field_has_its_keyword_and_value() {
        let field = first::<DateField>("date 2026-11-03\n");
        assert_eq!(field.keyword(), Keyword::Date);
        assert_eq!(
            field.value().map(|date| date.text().to_owned()).as_deref(),
            Some("2026-11-03")
        );
    }

    #[test]
    fn a_date_field_without_a_date_has_none() {
        assert_eq!(first::<DateField>("date\n").value(), None);
        assert_eq!(first::<DateField>("date \"November 3\"\n").value(), None);
    }

    #[test]
    fn only_a_date_field_node_is_a_date_field() {
        assert!(DateField::can_cast(SyntaxKind::DateField));
        assert!(!DateField::can_cast(SyntaxKind::StringField));
    }
}
