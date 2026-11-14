use oval_syntax::TextRange;
use oval_syntax::ast::{AstNode, Document, VersionMarker};

use super::{content_range, diagnostic};
use crate::{Diagnostic, ValidationRule};

pub(super) fn guide_fields(
    document: &Document,
    marker: &VersionMarker,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let line_end = content_range(marker.syntax())
        .expect("a marker begins with `oval`")
        .end();
    for absent in [
        document.guide().is_none(),
        document.election().is_none(),
        document.date().is_none(),
    ] {
        if absent {
            diagnostics.push(diagnostic(
                ValidationRule::MissingGuideField,
                TextRange::empty(line_end),
            ));
        }
    }
}

pub(super) fn root_ballots(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    if document.ballots().next().is_none() {
        let end = document.syntax().text_range().end();
        diagnostics.push(diagnostic(
            ValidationRule::EmptyDocument,
            TextRange::empty(end),
        ));
    }
}

#[cfg(test)]
mod tests {
    use crate::validate::{VALID, reported};

    fn to_u32(offset: usize) -> u32 {
        u32::try_from(offset).unwrap()
    }

    #[test]
    fn each_absent_guide_field_is_reported_at_the_end_of_the_markers_line() {
        assert_eq!(
            reported("oval 1  # c\n"),
            [
                ("missing-guide-field", 6, 6),
                ("missing-guide-field", 6, 6),
                ("missing-guide-field", 6, 6),
                ("empty-document", 12, 12),
            ]
        );
    }

    #[test]
    fn only_the_absent_field_is_reported() {
        let text = VALID.replacen("election \"E\"\n", "", 1);
        assert_eq!(reported(&text), [("missing-guide-field", 6, 6)]);
    }

    #[test]
    fn the_markers_line_ends_after_text_the_parser_discarded() {
        let text = VALID
            .replacen("oval 1", "oval 1 x", 1)
            .replacen("date 2026-11-03\n", "", 1);
        assert_eq!(
            reported(&text),
            [("unexpected-token", 7, 8), ("missing-guide-field", 8, 8)]
        );
    }

    #[test]
    fn a_field_without_a_value_is_present() {
        let text = VALID.replacen("guide \"G\"", "guide", 1);
        assert_eq!(reported(&text), [("missing-value", 12, 12)]);
    }

    #[test]
    fn a_misplaced_field_is_absent() {
        let text = VALID.replacen("date 2026-11-03\n", "", 1) + "date 2026-11-03\n";
        let start = to_u32(text.rfind("date").unwrap());
        assert_eq!(
            reported(&text),
            [
                ("missing-guide-field", 6, 6),
                ("misplaced-entry", start, start + 15),
            ]
        );
    }

    #[test]
    fn a_document_without_a_root_ballot_is_empty() {
        let text = &VALID[..VALID.find("ballot").unwrap()];
        let end = to_u32(text.len());
        assert_eq!(reported(text), [("empty-document", end, end)]);
    }
}
