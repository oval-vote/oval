use oval_syntax::ast::{AstNode, AstToken, DateLiteral, Document, StringLiteral, UrlLiteral};

use super::{diagnostic, kept_tokens};
use crate::{Diagnostic, ValidationRule};

pub(super) fn literals(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for token in kept_tokens(document.syntax()) {
        let range = token.text_range();
        let broken = if let Some(date) = DateLiteral::cast(token.clone()) {
            (!exists(&date)).then_some(ValidationRule::InvalidDate)
        } else if let Some(url) = UrlLiteral::cast(token.clone()) {
            url.text()
                .starts_with("http://")
                .then_some(ValidationRule::InsecureUrl)
        } else if let Some(string) = StringLiteral::cast(token) {
            is_blank(&string).then_some(ValidationRule::EmptyString)
        } else {
            None
        };
        if let Some(rule) = broken {
            diagnostics.push(diagnostic(rule, range));
        }
    }
}

fn exists(date: &DateLiteral) -> bool {
    let year = date.year();
    let is_leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match date.month() {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&date.day())
}

fn is_blank(string: &StringLiteral) -> bool {
    string.value().chars().all(|c| c == ' ' || c == '\t')
}

#[cfg(test)]
mod tests {
    use crate::validate::{VALID, reported, reported_as};

    fn with_date(date: &str) -> Vec<(u32, u32)> {
        reported_as(&VALID.replacen("2026-11-03", date, 1), "invalid-date")
    }

    fn with_guide(guide: &str) -> Vec<(u32, u32)> {
        reported_as(&VALID.replacen("\"G\"", guide, 1), "empty-string")
    }

    fn with_website(url: &str) -> Vec<(u32, u32)> {
        let text = VALID.replacen(
            "ballot",
            &format!("party @p \"P\"\n  website {url}\nballot"),
            1,
        );
        reported_as(&text, "insecure-url")
    }

    #[test]
    fn a_date_on_the_calendar_is_valid() {
        for date in [
            "2026-11-03",
            "2026-01-31",
            "2026-12-31",
            "2026-04-30",
            "0001-01-01",
        ] {
            assert_eq!(with_date(date), [], "{date}");
        }
    }

    #[test]
    fn a_month_outside_one_through_twelve_is_invalid() {
        assert_eq!(with_date("2026-00-10"), [(35, 45)]);
        assert_eq!(with_date("2026-13-10"), [(35, 45)]);
    }

    #[test]
    fn a_day_beyond_the_months_length_is_invalid() {
        assert_eq!(with_date("2026-11-00"), [(35, 45)]);
        assert_eq!(with_date("2026-04-31"), [(35, 45)]);
        assert_eq!(with_date("2026-01-32"), [(35, 45)]);
    }

    #[test]
    fn february_has_a_29th_only_in_a_leap_year() {
        assert_eq!(with_date("2028-02-29"), []);
        assert_eq!(with_date("2000-02-29"), []);
        assert_eq!(with_date("0000-02-29"), []);
        assert_eq!(with_date("2026-02-29"), [(35, 45)]);
        assert_eq!(with_date("1900-02-29"), [(35, 45)]);
        assert_eq!(with_date("2028-02-30"), [(35, 45)]);
    }

    #[test]
    fn an_invalid_date_is_an_error() {
        let report = crate::check(&VALID.replacen("2026-11-03", "2026-02-30", 1)).unwrap();
        assert!(report.has_errors());
    }

    #[test]
    fn a_blank_string_is_empty_including_its_quotes() {
        assert_eq!(with_guide("\"\""), [(13, 15)]);
        assert_eq!(with_guide("\" \t \""), [(13, 18)]);
    }

    #[test]
    fn a_string_with_any_other_character_is_not_empty() {
        assert_eq!(with_guide("\" a \""), []);
        assert_eq!(with_guide("\"\\\\\""), []);
        assert_eq!(with_guide("\"\u{a0}\""), []);
    }

    #[test]
    fn an_http_url_is_insecure() {
        assert_eq!(with_website("http://p.example"), [(69, 85)]);
        assert_eq!(with_website("https://p.example"), []);
    }

    #[test]
    fn a_warning_leaves_the_document_valid() {
        let report = crate::check(&VALID.replacen("\"G\"", "\"\"", 1)).unwrap();
        assert_eq!(report.diagnostics().len(), 1);
        assert!(!report.has_errors());
    }

    #[test]
    fn discarded_text_is_not_validated() {
        let text = VALID.replacen("date 2026-11-03", "date 2026-11-03 2026-02-30 \"\"", 1);
        assert_eq!(
            reported(&text)
                .iter()
                .map(|(id, _, _)| *id)
                .collect::<Vec<_>>(),
            ["unexpected-token"]
        );
    }

    #[test]
    fn set_aside_text_is_not_validated() {
        let text = VALID.replacen(
            "date 2026-11-03\n",
            "date 2026-11-03\ndate 2026-02-30\nguide \"\"\n",
            1,
        );
        assert_eq!(
            reported(&text)
                .iter()
                .map(|(id, _, _)| *id)
                .collect::<Vec<_>>(),
            ["repeated-field", "repeated-field"]
        );
    }
}
