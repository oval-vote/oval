use crate::ast::{AstToken, ast_token};

ast_token! {
    /// A date, such as `2026-11-03`, written as a four-digit year, a two-digit month, and a
    /// two-digit day.
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

#[cfg(test)]
mod tests {
    use super::DateLiteral;
    use crate::SyntaxKind;
    use crate::ast::AstToken;
    use crate::ast::literals::tokens;

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
}
