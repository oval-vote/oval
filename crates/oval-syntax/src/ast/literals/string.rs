use std::borrow::Cow;

use crate::ast::{AstToken, ast_token};

ast_token! {
    /// A string, such as `"Franklin Voter Guide"`, written in double quotes on one line.
    StringLiteral(String)
}

impl StringLiteral {
    /// Returns the string's value, the text between its quotes with each escape resolved.
    ///
    /// `\"` becomes a double quote and `\\` becomes a backslash. Any other backslash is an invalid
    /// escape, which the lexer reports, and is kept as written along with the character after it.
    /// The value borrows from the document unless the string has an escape to resolve.
    ///
    /// ```
    /// use oval_syntax::ast::{AstToken, StringLiteral};
    ///
    /// let result = oval_syntax::parse("guide \"Say \\\"yes\\\"\"\n")?;
    /// let string = result
    ///     .syntax()
    ///     .descendants_with_tokens()
    ///     .find_map(|element| element.into_token().and_then(StringLiteral::cast))
    ///     .expect("the guide field has a string");
    /// assert_eq!(string.text(), "\"Say \\\"yes\\\"\"");
    /// assert_eq!(string.value(), "Say \"yes\"");
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    pub fn value(&self) -> Cow<'_, str> {
        let text = self.text();
        let inner = &text[1..text.len() - 1];
        if !inner.contains('\\') {
            return Cow::Borrowed(inner);
        }
        let mut value = String::with_capacity(inner.len());
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\'
                && let Some(escaped) = chars.next_if(|&next| next == '"' || next == '\\')
            {
                value.push(escaped);
            } else {
                value.push(c);
            }
        }
        Cow::Owned(value)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::StringLiteral;
    use crate::ast::AstToken;
    use crate::ast::literals::tokens;
    use crate::{SyntaxKind, parse};

    fn string(value: &str) -> StringLiteral {
        tokens(&format!("guide {value}\n"))
            .into_iter()
            .find_map(StringLiteral::cast)
            .expect("the guide field has a string")
    }

    #[test]
    fn a_string_without_escapes_borrows_its_value() {
        let string = string("\"Franklin Voter Guide\"");
        assert!(matches!(
            string.value(),
            Cow::Borrowed("Franklin Voter Guide")
        ));
    }

    #[test]
    fn escapes_are_resolved() {
        assert_eq!(string(r#""a\"b""#).value(), "a\"b");
        assert_eq!(string(r#""a\\b""#).value(), "a\\b");
        assert_eq!(string(r#""\\\"""#).value(), "\\\"");
    }

    #[test]
    fn escaped_backslashes_are_resolved_in_pairs() {
        assert_eq!(string(r#""\\\\""#).value(), "\\\\");
    }

    #[test]
    fn an_invalid_escape_is_kept_as_written() {
        let result = parse("guide \"a\\#b\"\n").unwrap();
        assert_eq!(result.lexical_errors.len(), 1);
        assert_eq!(string(r#""a\#b""#).value(), "a\\#b");
    }

    #[test]
    fn an_empty_string_has_an_empty_value() {
        assert_eq!(string("\"\"").value(), "");
    }

    #[test]
    fn a_string_keeps_its_text_as_written() {
        assert_eq!(string(r#""a\"b""#).text(), r#""a\"b""#);
    }

    #[test]
    fn only_a_string_token_is_a_string_literal() {
        assert!(StringLiteral::can_cast(SyntaxKind::String));
        let word = tokens("guide \"G\"\n")
            .into_iter()
            .find(|token| token.kind() == SyntaxKind::Word)
            .expect("the field has a keyword");
        assert_eq!(StringLiteral::cast(word), None);
    }
}
