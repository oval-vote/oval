use crate::Keyword;
use crate::ast::{AstNode, MarkdownBody, ast_node, support};

ast_node! {
    /// A field whose value is a Markdown body, such as `note:` and the indented lines below it.
    ///
    /// ```
    /// let result = oval_syntax::parse("note:\n  Check your *own* ballot.\n")?;
    /// let note = result.document().note().expect("the document has a note");
    /// let body = note.body().expect("the note has a body");
    /// assert_eq!(body.value(), "Check your *own* ballot.");
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    MarkdownField
}

impl MarkdownField {
    /// Returns the field's keyword, such as [`Keyword::Note`].
    ///
    /// # Panics
    ///
    /// Never, since the parser builds a field only once it has read the field's keyword.
    pub fn keyword(&self) -> Keyword {
        support::keyword(self.syntax()).expect("a field begins with its keyword")
    }

    /// Returns the field's body, or `None` if the field has none.
    ///
    /// A field has no body when the line after its `:` is not indented further, or when its line
    /// had an error and the body was discarded with it.
    pub fn body(&self) -> Option<MarkdownBody> {
        support::token(self.syntax())
    }
}

#[cfg(test)]
mod tests {
    use super::MarkdownField;
    use crate::ast::AstNode;
    use crate::ast::fields::first;
    use crate::{Keyword, SyntaxKind};

    fn body(text: &str) -> Option<String> {
        first::<MarkdownField>(text).body().map(|body| body.value())
    }

    #[test]
    fn a_markdown_field_has_its_keyword_and_body() {
        let field = first::<MarkdownField>("note:\n  Check.\n");
        assert_eq!(field.keyword(), Keyword::Note);
        assert_eq!(body("note:\n  Check.\n").as_deref(), Some("Check."));
    }

    #[test]
    fn a_field_without_a_body_has_none() {
        assert_eq!(body("note:\ndate 2026-11-03\n"), None);
        assert_eq!(body("note\n"), None);
    }

    #[test]
    fn a_body_discarded_with_its_line_is_not_the_body() {
        let result = crate::parse("note x:\n  Discarded.\n").unwrap();
        assert_eq!(result.syntax_errors.len(), 1);
        assert_eq!(body("note x:\n  Discarded.\n"), None);
    }

    #[test]
    fn only_a_markdown_field_node_is_a_markdown_field() {
        assert!(MarkdownField::can_cast(SyntaxKind::MarkdownField));
        assert!(!MarkdownField::can_cast(SyntaxKind::StringField));
    }
}
