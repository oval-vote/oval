use crate::ast::{AstToken, ast_token};

ast_token! {
    /// An integer, such as `5`, written as one or more digits with no sign.
    IntegerLiteral(Integer)
}

impl IntegerLiteral {
    /// Returns the integer's value, or `None` if it is too large for a `u64`.
    ///
    /// Leading zeros do not change the value, so `007` is 7. The specification sets no upper bound
    /// on an integer, so a value too large to read is left to validation to report.
    ///
    /// ```
    /// use oval_syntax::ast::{AstToken, IntegerLiteral};
    ///
    /// let result = oval_syntax::parse("oval 007\n")?;
    /// let integer = result
    ///     .syntax()
    ///     .descendants_with_tokens()
    ///     .find_map(|element| element.into_token().and_then(IntegerLiteral::cast))
    ///     .expect("the version marker has an integer");
    /// assert_eq!(integer.text(), "007");
    /// assert_eq!(integer.value(), Some(7));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    pub fn value(&self) -> Option<u64> {
        self.text().parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::IntegerLiteral;
    use crate::SyntaxKind;
    use crate::ast::AstToken;
    use crate::ast::literals::tokens;

    fn integer(digits: &str) -> IntegerLiteral {
        tokens(&format!("oval {digits}\n"))
            .into_iter()
            .find_map(IntegerLiteral::cast)
            .expect("the version marker has an integer")
    }

    #[test]
    fn an_integer_has_the_value_of_its_digits() {
        assert_eq!(integer("5").value(), Some(5));
        assert_eq!(integer("0").value(), Some(0));
        assert_eq!(integer("2026").value(), Some(2026));
    }

    #[test]
    fn leading_zeros_do_not_change_the_value() {
        assert_eq!(integer("007").value(), Some(7));
        assert_eq!(integer("000").value(), Some(0));
        assert_eq!(integer("007").text(), "007");
    }

    #[test]
    fn the_largest_u64_has_a_value() {
        assert_eq!(integer("18446744073709551615").value(), Some(u64::MAX));
        assert_eq!(integer("0018446744073709551615").value(), Some(u64::MAX));
    }

    #[test]
    fn an_integer_too_large_for_a_u64_has_no_value() {
        assert_eq!(integer("18446744073709551616").value(), None);
        assert_eq!(integer("123456789012345678901234567890").value(), None);
    }

    #[test]
    fn only_an_integer_token_is_an_integer_literal() {
        assert!(IntegerLiteral::can_cast(SyntaxKind::Integer));
        assert!(!IntegerLiteral::can_cast(SyntaxKind::String));
        let word = tokens("oval 1\n")
            .into_iter()
            .find(|token| token.kind() == SyntaxKind::Word)
            .expect("the marker has a keyword");
        assert_eq!(IntegerLiteral::cast(word), None);
    }
}
