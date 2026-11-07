//! Line splitting shared by the parser, the formatter, and tools that read bodies.

/// Splits `text` into lines without their terminators, recognizing `\n`, `\r\n`, and `\r`.
/// A terminator at the very end does not produce a trailing empty line.
pub fn split_lines(text: &str) -> Vec<&str> {
    lines_with_offsets(text)
        .into_iter()
        .map(|(_, line)| line)
        .collect()
}

/// Splits `text` as [`split_lines`] does, pairing each line with its byte offset in `text`.
pub fn lines_with_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut lines = Vec::new();
    let mut start = 0;
    while let Some(i) = text[start..].find(['\n', '\r']) {
        let end = start + i;
        lines.push((start, &text[start..end]));
        start = if text[end..].starts_with("\r\n") {
            end + 2
        } else {
            end + 1
        };
    }
    if start < text.len() {
        lines.push((start, &text[start..]));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{lines_with_offsets, split_lines};

    #[test]
    fn every_terminator_form_splits_and_a_final_one_adds_nothing() {
        assert_eq!(split_lines("a\nb\r\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(split_lines("a\n"), ["a"]);
        assert_eq!(split_lines("a\n\n"), ["a", ""]);
        assert_eq!(split_lines(""), [] as [&str; 0]);
        assert_eq!(
            lines_with_offsets("ab\r\ncd\re"),
            [(0, "ab"), (4, "cd"), (7, "e")]
        );
    }
}
