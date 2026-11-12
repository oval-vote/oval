use rowan::{GreenNodeBuilder, Language};

use crate::{InputRejection, OvalLanguage, SyntaxKind, SyntaxNode, lex};

/// Parses an Oval document into a syntax tree.
///
/// The tree holds every token of the document, so its text is always exactly `text`. For now, every
/// token is a child of one [`SyntaxKind::Document`] node.
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
/// let text = "date 2026-11-03\n";
/// let tree = parse(text)?;
/// assert_eq!(tree.kind(), SyntaxKind::Document);
/// assert_eq!(tree.to_string(), text);
/// # Ok::<(), oval_syntax::InputRejection>(())
/// ```
pub fn parse(text: &str) -> Result<SyntaxNode, InputRejection> {
    let result = lex(text)?;
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(OvalLanguage::kind_to_raw(SyntaxKind::Document));
    let mut start = 0;
    for token in result.tokens {
        let end = start + token.len as usize;
        builder.token(
            OvalLanguage::kind_to_raw(token.kind.into()),
            &text[start..end],
        );
        start = end;
    }
    builder.finish_node();
    Ok(SyntaxNode::new_root(builder.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_round_trip(text: &str) {
        let tree = parse(text).expect("the text is accepted");
        assert_eq!(tree.to_string(), text);
    }

    #[test]
    fn the_root_is_a_document() {
        let tree = parse("date 2026-11-03\n").unwrap();
        assert_eq!(tree.kind(), SyntaxKind::Document);
    }

    #[test]
    fn every_token_is_a_child_of_the_document() {
        let text = "guide \"G\"\n  note:\n    Text.\n";
        let tree = parse(text).unwrap();
        let tokens = lex(text).unwrap().tokens;
        let children: Vec<SyntaxKind> = tree
            .children_with_tokens()
            .map(|child| child.kind())
            .collect();
        let expected: Vec<SyntaxKind> = tokens.iter().map(|token| token.kind.into()).collect();
        assert_eq!(children, expected);
    }

    #[test]
    fn empty_input_is_an_empty_document() {
        let tree = parse("").unwrap();
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
    fn a_rejected_document_is_not_parsed() {
        assert_eq!(
            parse("a\0b"),
            Err(InputRejection::NulCharacter { offset: 1 })
        );
    }
}
