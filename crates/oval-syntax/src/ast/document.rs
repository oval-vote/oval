use super::ast_node;

ast_node! {
    /// The root of a syntax tree, a whole Oval document.
    ///
    /// Every tree has exactly one, and [`ParseResult::document`](crate::ParseResult::document)
    /// returns it.
    Document
}

#[cfg(test)]
mod tests {
    use super::Document;
    use crate::ast::AstNode;
    use crate::{SyntaxKind, parse};

    #[test]
    fn the_root_is_a_document() {
        let root = parse("guide \"G\"\n").unwrap().syntax();
        let document = Document::cast(root.clone()).expect("the root is a document");
        assert_eq!(document.syntax(), &root);
    }

    #[test]
    fn another_node_is_not_a_document() {
        let field = parse("guide \"G\"\n")
            .unwrap()
            .syntax()
            .first_child()
            .expect("the document has a field");
        assert_eq!(field.kind(), SyntaxKind::StringField);
        assert_eq!(Document::cast(field), None);
    }

    #[test]
    fn only_a_document_kind_can_be_a_document() {
        assert!(Document::can_cast(SyntaxKind::Document));
        for kind in [SyntaxKind::Ballot, SyntaxKind::ErrorNode, SyntaxKind::Word] {
            assert!(!Document::can_cast(kind), "{kind:?}");
        }
    }

    #[test]
    fn a_parse_result_returns_its_document() {
        let result = parse("").unwrap();
        assert_eq!(result.document().syntax(), &result.syntax());
    }
}
