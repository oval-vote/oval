//! Value checks shared by the lowering pass.

#[allow(clippy::many_single_char_names)]
pub fn is_full_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| s[r].parse::<u32>().ok();
    let (Some(y), Some(m), Some(d)) = (num(0..4), num(5..7), num(8..10)) else {
        return false;
    };
    if !b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { true } else { c.is_ascii_digit() }) {
        return false;
    }
    if !(1..=12).contains(&m) || d == 0 {
        return false;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if leap => 29,
        _ => 28,
    };
    d <= max
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
