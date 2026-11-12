use std::borrow::Cow;

use super::{AstToken, ast_token};

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

ast_token! {
    /// A date, such as `2026-11-03`, written as a four-digit year, a two-digit month, and a two-digit
    /// day.
    ///
    /// The parts are read as written. Whether the date exists on the calendar, which `2026-02-30`
    /// does not, is left to validation.
    ///
    /// ```
    /// use oval_syntax::ast::{AstToken, DateLiteral};
    ///
    /// let result = oval_syntax::parse("date 2026-11-03\n")?;
    /// let date = result
    ///     .syntax()
    ///     .descendants_with_tokens()
    ///     .find_map(|element| element.into_token().and_then(DateLiteral::cast))
    ///     .expect("the date field has a date");
    /// assert_eq!((date.year(), date.month(), date.day()), (2026, 11, 3));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    DateLiteral(Date)
}

impl DateLiteral {
    /// Returns the year, the first four digits.
    pub fn year(&self) -> u16 {
        u16::from(self.two_digits(0)) * 100 + u16::from(self.two_digits(2))
    }

    /// Returns the month, the two digits after the first `-`, from 0 to 99.
    pub fn month(&self) -> u8 {
        self.two_digits(5)
    }

    /// Returns the day, the last two digits, from 0 to 99.
    pub fn day(&self) -> u8 {
        self.two_digits(8)
    }

    fn two_digits(&self, start: usize) -> u8 {
        let bytes = self.text().as_bytes();
        (bytes[start] - b'0') * 10 + (bytes[start + 1] - b'0')
    }
}

ast_token! {
    /// A URL, such as `https://dems.example`, written without quotes.
    ///
    /// Its value is its [`text`](AstToken::text), from the `http` or `https` scheme to the next space
    /// or the end of the line.
    UrlLiteral(Url)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{DateLiteral, IntegerLiteral, StringLiteral, UrlLiteral};
    use crate::ast::AstToken;
    use crate::{SyntaxKind, SyntaxToken, parse};

    fn tokens(text: &str) -> Vec<SyntaxToken> {
        parse(text)
            .unwrap()
            .syntax()
            .descendants_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .collect()
    }

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

    fn date(text: &str) -> DateLiteral {
        tokens(&format!("date {text}\n"))
            .into_iter()
            .find_map(DateLiteral::cast)
            .expect("the date field has a date")
    }

    fn parts(date: &DateLiteral) -> (u16, u8, u8) {
        (date.year(), date.month(), date.day())
    }

    #[test]
    fn a_date_has_a_year_a_month_and_a_day() {
        assert_eq!(parts(&date("2026-11-03")), (2026, 11, 3));
        assert_eq!(parts(&date("9999-12-31")), (9999, 12, 31));
    }

    #[test]
    fn leading_zeros_in_a_date_do_not_change_its_parts() {
        assert_eq!(parts(&date("0001-01-09")), (1, 1, 9));
        assert_eq!(parts(&date("0000-00-00")), (0, 0, 0));
    }

    #[test]
    fn a_date_not_on_the_calendar_is_read_as_written() {
        assert_eq!(parts(&date("2026-02-30")), (2026, 2, 30));
        assert_eq!(parts(&date("2026-13-99")), (2026, 13, 99));
    }

    #[test]
    fn a_date_keeps_its_text_as_written() {
        assert_eq!(date("2026-11-03").text(), "2026-11-03");
    }

    #[test]
    fn only_a_date_token_is_a_date_literal() {
        assert!(DateLiteral::can_cast(SyntaxKind::Date));
        assert!(!DateLiteral::can_cast(SyntaxKind::Integer));
        let word = tokens("date 2026-11-03\n")
            .into_iter()
            .find(|token| token.kind() == SyntaxKind::Word)
            .expect("the field has a keyword");
        assert_eq!(DateLiteral::cast(word), None);
    }

    #[test]
    fn a_url_is_its_text() {
        let url = tokens("website https://dems.example/join?ref=guide#top\n")
            .into_iter()
            .find_map(UrlLiteral::cast)
            .expect("the website field has a URL");
        assert_eq!(url.text(), "https://dems.example/join?ref=guide#top");
    }

    #[test]
    fn only_a_url_token_is_a_url_literal() {
        assert!(UrlLiteral::can_cast(SyntaxKind::Url));
        assert!(!UrlLiteral::can_cast(SyntaxKind::String));
        let word = tokens("website https://dems.example\n")
            .into_iter()
            .find(|token| token.kind() == SyntaxKind::Word)
            .expect("the field has a keyword");
        assert_eq!(UrlLiteral::cast(word), None);
    }
}
