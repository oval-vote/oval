use crate::Keyword;
use crate::ast::{AstNode, UrlLiteral, ast_node, support};

ast_node! {
    /// A field whose value is a URL, such as `website https://dems.example`.
    UrlField
}

impl UrlField {
    /// Returns the field's keyword, such as [`Keyword::Website`].
    ///
    /// # Panics
    ///
    /// Never, since the parser builds a field only once it has read the field's keyword.
    pub fn keyword(&self) -> Keyword {
        support::keyword(self.syntax()).expect("a field begins with its keyword")
    }

    /// Returns the field's URL, or `None` if the line ends without one or has another kind of token
    /// in its place.
    pub fn value(&self) -> Option<UrlLiteral> {
        support::token(self.syntax())
    }
}

#[cfg(test)]
mod tests {
    use super::UrlField;
    use crate::ast::fields::first;
    use crate::ast::{AstNode, AstToken};
    use crate::{Keyword, SyntaxKind};

    #[test]
    fn a_url_field_has_its_keyword_and_value() {
        let field = first::<UrlField>("party @d \"D\"\n  website https://dems.example\n");
        assert_eq!(field.keyword(), Keyword::Website);
        assert_eq!(
            field.value().map(|url| url.text().to_owned()).as_deref(),
            Some("https://dems.example")
        );
    }

    #[test]
    fn a_url_field_without_a_url_has_none() {
        assert_eq!(
            first::<UrlField>("party @d \"D\"\n  website\n").value(),
            None
        );
        assert_eq!(
            first::<UrlField>("party @d \"D\"\n  website \"dems.example\"\n").value(),
            None
        );
    }

    #[test]
    fn only_a_url_field_node_is_a_url_field() {
        assert!(UrlField::can_cast(SyntaxKind::UrlField));
        assert!(!UrlField::can_cast(SyntaxKind::StringField));
    }
}
