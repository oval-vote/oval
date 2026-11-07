use oval_syntax::ast::{Date, Document};

use super::report;
use crate::{Diagnostic, ValidationRule};

pub(crate) fn check(document: &Document, out: &mut Vec<Diagnostic>) {
    let Some(field) = document.date() else {
        return;
    };
    let Some(date) = field.value() else {
        return;
    };
    if !is_calendar_date(date) {
        let Some(range) = field.value_range() else {
            return;
        };
        out.push(report(
            ValidationRule::InvalidDate,
            range,
            format!(
                "{:04}-{:02}-{:02} is not a calendar date",
                date.year, date.month, date.day
            ),
        ));
    }
}

fn is_calendar_date(date: Date) -> bool {
    let days_in_month = match date.month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(date.year) => 29,
        2 => 28,
        _ => return false,
    };
    date.day >= 1 && date.day <= days_in_month
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}
