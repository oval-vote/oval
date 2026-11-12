use crate::Keyword;
use crate::ast::{AstNode, StringLiteral, ast_node, support};

ast_node! {
    /// A field whose value is a list of strings separated by commas, such as
    /// `supporters "Franklin Housing Coalition", "Franklin Labor Federation"`, or the word `none`.
    ///
    /// The field has three states. It is absent when no one has looked, `none` when the list is
    /// known to be empty, and a list of strings otherwise.
    StringListField
}

impl StringListField {
    /// Returns the field's keyword, such as [`Keyword::Supporters`].
    ///
    /// # Panics
    ///
    /// Never, since the parser builds a field only once it has read the field's keyword.
    pub fn keyword(&self) -> Keyword {
        support::keyword(self.syntax()).expect("a field begins with its keyword")
    }

    /// Returns the field's strings in the order written.
    ///
    /// There are none when the field is written as `none`, or when its line ends without a value.
    pub fn strings(&self) -> impl Iterator<Item = StringLiteral> + use<> {
        support::tokens(self.syntax())
    }

    /// Returns `true` if the field is written as `none`, a list known to be empty.
    pub fn is_none_value(&self) -> bool {
        support::has_keyword(self.syntax(), Keyword::NoneValue)
    }
}

#[cfg(test)]
mod tests {
    use super::StringListField;
    use crate::ast::AstNode;
    use crate::ast::fields::first;
    use crate::{Keyword, SyntaxKind};

    fn field(line: &str) -> StringListField {
        first(&format!(
            "ballot @a \"A\"\n  measure @p \"P\"\n    {line}\n"
        ))
    }

    fn strings(field: &StringListField) -> Vec<String> {
        field
            .strings()
            .map(|string| string.value().into_owned())
            .collect()
    }

    #[test]
    fn a_list_has_its_strings_in_order() {
        let field = field("supporters \"A\", \"B\", \"C\"");
        assert_eq!(field.keyword(), Keyword::Supporters);
        assert_eq!(strings(&field), ["A", "B", "C"]);
        assert!(!field.is_none_value());
    }

    #[test]
    fn none_is_a_list_known_to_be_empty() {
        let field = field("opponents none");
        assert_eq!(field.keyword(), Keyword::Opponents);
        assert_eq!(strings(&field), Vec::<String>::new());
        assert!(field.is_none_value());
    }

    #[test]
    fn none_after_strings_is_skipped_and_does_not_count() {
        let field = field("supporters \"A\", none");
        assert_eq!(strings(&field), ["A"]);
        assert!(!field.is_none_value());
    }

    #[test]
    fn a_list_without_a_value_is_empty_but_not_none() {
        let field = field("supporters");
        assert_eq!(strings(&field), Vec::<String>::new());
        assert!(!field.is_none_value());
    }

    #[test]
    fn only_a_string_list_field_node_is_a_string_list_field() {
        assert!(StringListField::can_cast(SyntaxKind::StringListField));
        assert!(!StringListField::can_cast(SyntaxKind::StringField));
    }
}
