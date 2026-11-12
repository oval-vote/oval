mod parse_result;
mod syntax_error;
mod token_parser;

use token_parser::TokenParser;

pub use parse_result::ParseResult;
pub use syntax_error::{Expected, SyntaxError, SyntaxErrorKind};

use crate::{InputRejection, LexResult, lex};

/// Parses an Oval document into a syntax tree.
///
/// The tree holds every token of the document, so its text is always exactly `text`. Parsing never
/// stops at an error. The tree is complete even when the document has lexical or syntax errors,
/// which the [`ParseResult`] reports alongside it.
///
/// Only the version marker is parsed so far. Every other token is a direct child of the
/// [`Document`](crate::SyntaxKind::Document) node.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `text` cannot be lexed at all. See [`lex`].
///
/// # Examples
///
/// ```
/// use oval_syntax::{SyntaxKind, parse};
///
/// let text = "oval 1\n";
/// let result = parse(text)?;
/// let tree = result.syntax();
/// assert_eq!(tree.kind(), SyntaxKind::Document);
/// assert_eq!(tree.to_string(), text);
/// assert!(result.lexical_errors.is_empty());
/// assert!(result.syntax_errors.is_empty());
/// # Ok::<(), oval_syntax::InputRejection>(())
/// ```
pub fn parse(text: &str) -> Result<ParseResult, InputRejection> {
    let LexResult { tokens, errors } = lex(text)?;
    let mut parser = TokenParser::new(text, tokens);
    parser.document();
    Ok(ParseResult {
        green: parser.finish(),
        lexical_errors: errors,
        syntax_errors: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LexicalError, LexicalErrorKind, SyntaxKind};
    use expect_test::{Expect, expect};

    pub(super) fn check(text: &str, expected: &Expect) {
        let tree = parse(text).expect("the text is accepted").syntax();
        expected.assert_eq(&format!("{tree:#?}"));
    }

    fn check_round_trip(text: &str) {
        let tree = parse(text).expect("the text is accepted").syntax();
        assert_eq!(tree.to_string(), text);
    }

    #[test]
    fn the_root_is_a_document() {
        let tree = parse("date 2026-11-03\n").unwrap().syntax();
        assert_eq!(tree.kind(), SyntaxKind::Document);
    }

    #[test]
    fn a_clean_document_has_no_errors() {
        let result = parse("oval 1\nguide \"G\"\n").unwrap();
        assert_eq!(result.lexical_errors, []);
        assert_eq!(result.syntax_errors, []);
    }

    #[test]
    fn lexical_errors_are_reported_alongside_the_tree() {
        let text = "subtitle\t\"G\"\n";
        let result = parse(text).unwrap();
        assert_eq!(
            result.lexical_errors,
            [LexicalError {
                kind: LexicalErrorKind::IllegalTab,
                offset: 8,
                len: 1,
            }]
        );
        assert_eq!(result.syntax_errors, []);
        assert_eq!(result.syntax().to_string(), text);
    }

    #[test]
    fn every_token_is_a_child_of_the_document() {
        check(
            "guide \"G\"\n  note:\n    Text.\n",
            &expect![[r#"
                Document@0..28
                  Word@0..5 "guide"
                  Whitespace@5..6 " "
                  String@6..9 "\"G\""
                  Newline@9..10 "\n"
                  Whitespace@10..12 "  "
                  Indent@12..12 ""
                  Word@12..16 "note"
                  Colon@16..17 ":"
                  Newline@17..18 "\n"
                  MarkdownBlock@18..28 "    Text.\n"
                  Dedent@28..28 ""
            "#]],
        );
    }

    #[test]
    fn empty_input_is_an_empty_document() {
        let tree = parse("").unwrap().syntax();
        assert_eq!(tree.kind(), SyntaxKind::Document);
        assert_eq!(tree.children_with_tokens().count(), 0);
    }

    #[test]
    fn the_tree_reproduces_the_document() {
        check_round_trip("guide \"Franklin County Voters' Guide\"\ndate 2026-11-03\n");
    }

    #[test]
    fn the_tree_reproduces_errors() {
        check_round_trip("subtitle\t\"Vote for one\n12abc @ ~\n");
    }

    #[test]
    fn the_tree_reproduces_layout_and_markdown() {
        check_round_trip("ballot \"B\"\n  contest \"C\"\n    note:\n      Read *both*.\n\nend");
    }

    #[test]
    fn the_tree_reproduces_line_terminators_and_a_byte_order_mark() {
        check_round_trip("\u{FEFF}a\r\n  b\rc\n\n# comment\n");
    }

    #[test]
    fn ignored_tokens_at_the_end_are_inside_the_document() {
        check(
            "word\n\n# last\n",
            &expect![[r##"
                Document@0..13
                  Word@0..4 "word"
                  Newline@4..5 "\n"
                  BlankLine@5..6 "\n"
                  Comment@6..12 "# last"
                  BlankLine@12..13 "\n"
            "##]],
        );
    }

    #[test]
    fn a_document_of_only_ignored_tokens_keeps_them() {
        check(
            "# just a comment\n",
            &expect![[r##"
                Document@0..17
                  Comment@0..16 "# just a comment"
                  BlankLine@16..17 "\n"
            "##]],
        );
    }

    #[test]
    fn a_rejected_document_is_not_parsed() {
        assert_eq!(
            parse("a\0b"),
            Err(InputRejection::NulCharacter { offset: 1 })
        );
    }
}
