use oval_syntax::{SyntaxKind, SyntaxToken};

use crate::entry::Entry;

const INDENT: &str = "  ";

pub(crate) fn print(entries: &[Entry], depth: usize, output: &mut String) {
    for entry in entries {
        output.push_str(&INDENT.repeat(depth));
        line(&entry.line, output);
        output.push('\n');
        if let Some(body) = &entry.body {
            output.push_str(body.text());
        }
        print(&entry.children, depth + 1, output);
    }
}

fn line(tokens: &[SyntaxToken], output: &mut String) {
    let mut previous = None;
    for token in tokens {
        let attached = matches!(token.kind(), SyntaxKind::Colon | SyntaxKind::Comma)
            || previous == Some(SyntaxKind::AtSign);
        if previous.is_some() && !attached {
            output.push(' ');
        }
        output.push_str(token.text());
        previous = Some(token.kind());
    }
}

#[cfg(test)]
mod tests {
    use crate::format;

    fn formatted(text: &str) -> String {
        format(text).unwrap()
    }

    #[test]
    fn each_line_is_indented_two_spaces_for_each_enclosing_block() {
        assert_eq!(
            formatted(
                "oval 1\nballot @b \"B\"\n    section \"S\"\n         contest @k \"K\" single\n          campaign @a \"A\"\n         measure @m \"M\"\n            recommend yes\n    ballot @c \"C\"\n       measure @n \"N\"\n        recommend no\n"
            ),
            "oval 1\nballot @b \"B\"\n  section \"S\"\n    contest @k \"K\" single\n      campaign @a \"A\"\n    measure @m \"M\"\n      recommend yes\n  ballot @c \"C\"\n    measure @n \"N\"\n      recommend no\n"
        );
    }

    #[test]
    fn tokens_are_separated_by_one_space_with_punctuation_attached() {
        assert_eq!(
            formatted(
                "oval   1\nballot  @b   \"B\"\n  measure @m \"M\"\n    supporters \"A\" ,\"B\",   \"C\"\n    note  :\n      Text.\n"
            ),
            "oval 1\nballot @b \"B\"\n  measure @m \"M\"\n    supporters \"A\", \"B\", \"C\"\n    note:\n      Text.\n"
        );
    }

    #[test]
    fn every_line_ends_with_a_line_feed() {
        assert_eq!(
            formatted("oval 1\r\nguide \"G\"\relection \"E\""),
            "oval 1\nguide \"G\"\nelection \"E\"\n"
        );
    }

    #[test]
    fn a_byte_order_mark_is_removed() {
        assert_eq!(formatted("\u{feff}oval 1\n"), "oval 1\n");
    }

    #[test]
    fn a_formatted_document_formats_to_itself() {
        let text = "oval 1\nguide \"G\"\nballot @b \"B\"\n  contest @k \"K\" ranked max 3\n    campaign\n      @a \"A\" as \"Lead\"\n      @c \"C\"\n    recommend\n      support @a\n      oppose write-in\n";
        assert_eq!(formatted(text), text);
    }
}
