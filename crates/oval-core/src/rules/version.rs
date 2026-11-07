use oval_syntax::ast::{AstNode, Document, VersionMarker};
use oval_syntax::{SUPPORTED_LANGUAGE_MAJOR, SyntaxKind, TextRange};

use super::report;
use crate::{Diagnostic, ValidationRule};

/// Checks the version marker.
///
/// Returns `false` when the marker is missing or names a major version this crate does not read,
/// in which case `unsupported-version` is the only diagnostic the document gets.
pub(crate) fn check(document: &Document, out: &mut Vec<Diagnostic>) -> bool {
    let Some(marker) = document.version() else {
        out.push(report(
            ValidationRule::UnsupportedVersion,
            TextRange::empty(first_content_offset(document)),
            format!(
                "the document has no version marker; its first line should be `oval {SUPPORTED_LANGUAGE_MAJOR}`"
            ),
        ));
        return false;
    };
    let Some(major) = marker.value() else {
        return true;
    };
    if major == SUPPORTED_LANGUAGE_MAJOR {
        return true;
    }
    out.push(report(
        ValidationRule::UnsupportedVersion,
        marker_range(&marker),
        format!(
            "this document is written to Oval {major}, but this implementation reads Oval {SUPPORTED_LANGUAGE_MAJOR}"
        ),
    ));
    false
}

fn first_content_offset(document: &Document) -> oval_syntax::TextSize {
    document
        .syntax()
        .descendants_with_tokens()
        .filter_map(oval_syntax::SyntaxElement::into_token)
        .find(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace
                    | SyntaxKind::Comment
                    | SyntaxKind::BlankLine
                    | SyntaxKind::Newline
            )
        })
        .map_or_else(|| 0.into(), |token| token.text_range().start())
}

fn marker_range(marker: &VersionMarker) -> TextRange {
    let end = marker
        .syntax()
        .children_with_tokens()
        .filter_map(oval_syntax::SyntaxElement::into_token)
        .find(|token| token.kind() == SyntaxKind::Newline)
        .map_or_else(|| marker.range().end(), |token| token.text_range().start());
    TextRange::new(marker.range().start(), end)
}
