use crate::Keyword;
use crate::ast::{
    AstNode, Candidate, DateField, MarkdownField, Party, StringField, VersionMarker, ast_node,
    support,
};

ast_node! {
    /// The root of a syntax tree, a whole Oval document.
    ///
    /// Every tree has exactly one, and [`ParseResult::document`](crate::ParseResult::document)
    /// returns it.
    ///
    /// Each accessor returns the entry that stands. An entry the parser set aside, because it
    /// repeats a field or comes after a group it belongs before, is not returned.
    ///
    /// ```
    /// let text = "oval 1\nguide \"Franklin Voter Guide\"\ndate 2026-11-03\n";
    /// let result = oval_syntax::parse(text)?;
    /// let document = result.document();
    /// assert!(document.version_marker().is_some());
    /// assert!(document.guide().is_some());
    /// assert!(document.election().is_none());
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    Document
}

impl Document {
    /// Returns the version marker, or `None` if the document does not begin with one.
    pub fn version_marker(&self) -> Option<VersionMarker> {
        support::child(self.syntax())
    }

    /// Returns the `guide` field, the guide's title, or `None` if the document has none.
    ///
    /// A field written without a value is still returned, with no [`value`](StringField::value).
    pub fn guide(&self) -> Option<StringField> {
        support::field(self.syntax(), Keyword::Guide)
    }

    /// Returns the `election` field, the election's name, or `None` if the document has none.
    pub fn election(&self) -> Option<StringField> {
        support::field(self.syntax(), Keyword::Election)
    }

    /// Returns the `date` field, the election's date, or `None` if the document has none.
    pub fn date(&self) -> Option<DateField> {
        support::child(self.syntax())
    }

    /// Returns the document's `note` field, or `None` if the document has none.
    pub fn note(&self) -> Option<MarkdownField> {
        support::child(self.syntax())
    }

    /// Returns the parties declared at the top level, in the order written.
    ///
    /// A party declared inline, in a candidate's `party` field or a party campaign, is not among
    /// them.
    pub fn parties(&self) -> impl Iterator<Item = Party> + use<> {
        support::children(self.syntax())
    }

    /// Returns the candidates declared at the top level, in the order written.
    ///
    /// A candidate declared inline, as a member of a campaign, is not among them.
    pub fn candidates(&self) -> impl Iterator<Item = Candidate> + use<> {
        support::children(self.syntax())
    }
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

    fn string_value(field: Option<crate::ast::StringField>) -> Option<String> {
        field
            .and_then(|field| field.value())
            .map(|value| value.value().into_owned())
    }

    #[test]
    fn a_document_has_its_guide_fields() {
        let result = parse(
            "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nnote:\n  N.\nparty @d \"D\"\n",
        )
        .unwrap();
        let document = result.document();
        let version = document
            .version_marker()
            .and_then(|marker| marker.version());
        assert_eq!(version.and_then(|version| version.value()), Some(1));
        assert_eq!(string_value(document.guide()).as_deref(), Some("G"));
        assert_eq!(string_value(document.election()).as_deref(), Some("E"));
        let date = document.date().and_then(|field| field.value());
        assert_eq!(date.map(|date| date.year()), Some(2026));
        let note = document.note().and_then(|field| field.body());
        assert_eq!(note.map(|body| body.value()).as_deref(), Some("N."));
    }

    #[test]
    fn an_empty_document_has_no_guide_fields() {
        let document = parse("").unwrap().document();
        assert_eq!(document.version_marker(), None);
        assert_eq!(document.guide(), None);
        assert_eq!(document.election(), None);
        assert_eq!(document.date(), None);
        assert_eq!(document.note(), None);
    }

    #[test]
    fn a_field_without_a_value_is_still_present() {
        let guide = parse("guide\n").unwrap().document().guide();
        assert!(guide.is_some_and(|field| field.value().is_none()));
    }

    #[test]
    fn a_repeated_field_is_not_the_field() {
        let document = parse("guide \"A\"\nguide \"B\"\n").unwrap().document();
        assert_eq!(string_value(document.guide()).as_deref(), Some("A"));
    }

    #[test]
    fn a_misplaced_entry_is_not_part_of_the_document() {
        assert_eq!(
            parse("party @d \"D\"\nguide \"G\"\n")
                .unwrap()
                .document()
                .guide(),
            None
        );
        assert_eq!(
            parse("guide \"G\"\noval 1\n")
                .unwrap()
                .document()
                .version_marker(),
            None
        );
    }

    #[test]
    fn a_note_inside_a_declaration_is_not_the_document_note() {
        let document = parse("party @d \"D\"\n  note:\n    P.\n")
            .unwrap()
            .document();
        assert_eq!(document.note(), None);
    }

    #[test]
    fn a_document_has_its_declarations_in_order() {
        let document =
            parse("party @dem \"D\"\ncandidate @a \"A\"\nparty @rep \"R\"\ncandidate @b \"B\"\n")
                .unwrap()
                .document();
        let parties: Vec<_> = document
            .parties()
            .filter_map(|party| party.identifier())
            .collect();
        let candidates: Vec<_> = document
            .candidates()
            .filter_map(|c| c.identifier())
            .collect();
        let names = |ids: Vec<crate::ast::Identifier>| -> Vec<String> {
            ids.iter()
                .filter_map(|id| id.name().map(|name| name.text().to_owned()))
                .collect()
        };
        assert_eq!(names(parties), ["dem", "rep"]);
        assert_eq!(names(candidates), ["a", "b"]);
    }

    #[test]
    fn misplaced_and_inline_declarations_are_not_the_document_declarations() {
        let document = parse(
            "candidate @a \"A\"\n  party @green \"Green\"\nballot @b \"B\"\n  eligibility \"E\"\nparty @late \"L\"\n",
        )
        .unwrap()
        .document();
        assert_eq!(document.parties().count(), 0);
        assert_eq!(document.candidates().count(), 1);
    }
}
