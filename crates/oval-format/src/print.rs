use oval_syntax::{SyntaxKind, SyntaxToken};

use crate::entry::{Document, Entry, Trivia};

const INDENT: &str = "  ";

pub(crate) fn print(document: &Document) -> String {
    let mut output = String::new();
    entries(&document.entries, 0, &mut output);
    for trivia in &document.trailing {
        self::trivia(trivia, 0, &mut output);
    }
    output
}

fn entries(entries: &[Entry], depth: usize, output: &mut String) {
    for entry in entries {
        for trivia in &entry.leading {
            self::trivia(trivia, depth, output);
        }
        output.push_str(&INDENT.repeat(depth));
        line(&entry.line, output);
        if let Some(comment) = &entry.comment {
            output.push(' ');
            output.push_str(comment);
        }
        output.push('\n');
        if let Some(body) = &entry.body {
            for trivia in &body.leading {
                self::trivia(trivia, depth + 1, output);
            }
            for line in body.value.split('\n') {
                if !line.is_empty() {
                    output.push_str(&INDENT.repeat(depth + 1));
                    output.push_str(line);
                }
                output.push('\n');
            }
        }
        self::entries(&entry.children, depth + 1, output);
    }
}

fn trivia(trivia: &Trivia, depth: usize, output: &mut String) {
    if let Trivia::Comment(comment) = trivia {
        output.push_str(&INDENT.repeat(depth));
        output.push_str(comment);
    }
    output.push('\n');
}

fn line(tokens: &[SyntaxToken], output: &mut String) {
    let mut previous = None;
    for token in tokens {
        let attached = matches!(token.kind(), SyntaxKind::Colon | SyntaxKind::Comma)
            || previous == Some(SyntaxKind::AtSign);
        if previous.is_some() && !attached {
            output.push(' ');
        }
        if token.kind() == SyntaxKind::Integer {
            output.push_str(without_leading_zeros(token.text()));
        } else {
            output.push_str(token.text());
        }
        previous = Some(token.kind());
    }
}

fn without_leading_zeros(integer: &str) -> &str {
    match integer.trim_start_matches('0') {
        "" => "0",
        digits => digits,
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
    fn an_integer_is_written_without_leading_zeros() {
        assert_eq!(
            formatted(
                "oval 01\nballot @b \"B\"\n  contest @k \"K\" multiple 03 min 002\n    campaign @a \"A\"\n  contest @r \"R\" ranked max 10\n    campaign @c \"C\"\n  contest @z \"Z\" multiple 000\n    campaign @d \"D\"\n"
            ),
            "oval 1\nballot @b \"B\"\n  contest @k \"K\" multiple 3 min 2\n    campaign @a \"A\"\n  contest @r \"R\" ranked max 10\n    campaign @c \"C\"\n  contest @z \"Z\" multiple 0\n    campaign @d \"D\"\n"
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
    fn a_body_is_one_level_deeper_than_its_keyword_and_keeps_its_own_indentation() {
        assert_eq!(
            formatted(
                "oval 1\nnote:\n        Lists:\n\n          - one\n            more\n        - two\nballot @b \"B\"\n      note:\n       Text.\n      measure @m \"M\"\n        recommend yes\n            justification:\n                 Why.\n"
            ),
            "oval 1\nnote:\n  Lists:\n\n    - one\n      more\n  - two\nballot @b \"B\"\n  note:\n    Text.\n  measure @m \"M\"\n    recommend yes\n      justification:\n        Why.\n"
        );
    }

    #[test]
    fn blank_lines_inside_a_body_are_kept_and_whitespace_only_ones_become_empty() {
        assert_eq!(
            formatted("oval 1\nnote:\n    One.\n\n      \n\n    Two.\n\n\nguide \"G\"\n"),
            "oval 1\nnote:\n  One.\n\n\n\n  Two.\n\nguide \"G\"\n"
        );
    }

    #[test]
    fn a_body_keeps_trailing_spaces_and_tabs_as_text() {
        assert_eq!(
            formatted("oval 1\nnote:\r\n    Hard  \r\n    \tbreak.   \r    End.\nguide \"G\"\n"),
            "oval 1\nnote:\n  Hard  \n  \tbreak.   \n  End.\nguide \"G\"\n"
        );
    }

    #[test]
    fn a_body_at_the_end_of_the_document_ends_with_a_line_feed() {
        assert_eq!(
            formatted("oval 1\nnote:\n   Last."),
            "oval 1\nnote:\n  Last.\n"
        );
    }

    #[test]
    fn a_blank_line_before_a_body_stays_with_it() {
        assert_eq!(
            formatted("oval 1\nnote:\n\n\n    Text.\nguide \"G\"\n"),
            "oval 1\nnote:\n\n  Text.\nguide \"G\"\n"
        );
    }

    #[test]
    fn a_formatted_document_formats_to_itself() {
        let text = "oval 1\nguide \"G\"\nballot @b \"B\"\n  contest @k \"K\" ranked max 3\n    campaign\n      @a \"A\" as \"Lead\"\n      @c \"C\"\n    recommend\n      support @a\n      oppose write-in\n";
        assert_eq!(formatted(text), text);
    }
}
