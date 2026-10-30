//! Value checks shared by the lowering pass.

/// RFC 3339 full-date: `YYYY-MM-DD`, with the day checked against the month
/// and leap years.
pub fn is_full_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return false;
    }
    let num = |r: std::ops::Range<usize>| s[r].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (num(0..4), num(5..7), num(8..10)) else {
        return false;
    };
    if !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if leap => 29,
        _ => 28,
    };
    day <= days_in_month
}

pub fn is_http_url(s: &str) -> bool {
    let rest = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://"));
    matches!(rest, Some(r) if !r.is_empty() && !r.starts_with('/') && !r.contains(char::is_whitespace))
}

pub fn is_language_tag(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|p| !p.is_empty() && p.len() <= 8 && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates() {
        assert!(is_full_date("2026-11-03"));
        assert!(is_full_date("2024-02-29"));
        assert!(!is_full_date("2025-02-29"));
        assert!(!is_full_date("2026-13-01"));
        assert!(!is_full_date("26-11-03"));
        assert!(!is_full_date("2026-11-3"));
    }
    #[test]
    fn urls() {
        assert!(is_http_url("https://example.org/x"));
        assert!(!is_http_url("example.org"));
        assert!(!is_http_url("ftp://example.org"));
        assert!(!is_http_url("https://"));
    }
}
