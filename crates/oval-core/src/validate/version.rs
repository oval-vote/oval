use oval_syntax::TextRange;
use oval_syntax::ast::{AstNode, Document, VersionMarker};

use super::{content_range, diagnostic};
use crate::{Diagnostic, ValidationRule};

const MAJOR_VERSION: u64 = 1;

pub(super) fn supported_marker(document: &Document) -> Result<VersionMarker, Diagnostic> {
    let Some(marker) = document.version_marker() else {
        let syntax = document.syntax();
        let start = content_range(syntax).map_or(syntax.text_range().end(), TextRange::start);
        return Err(unsupported(TextRange::empty(start)));
    };
    let version = marker.version().and_then(|version| version.value());
    if version != Some(MAJOR_VERSION) {
        let range = content_range(marker.syntax()).expect("a marker begins with `oval`");
        return Err(unsupported(range));
    }
    Ok(marker)
}

fn unsupported(range: TextRange) -> Diagnostic {
    diagnostic(ValidationRule::UnsupportedVersion, range)
}

#[cfg(test)]
mod tests {
    use crate::validate::{VALID, reported};

    fn unsupported(start: u32, end: u32) -> Vec<(&'static str, u32, u32)> {
        vec![("unsupported-version", start, end)]
    }

    #[test]
    fn a_version_written_with_leading_zeros_is_supported() {
        assert_eq!(reported(&VALID.replacen("oval 1", "oval 01", 1)), []);
    }

    #[test]
    fn another_major_version_is_unsupported() {
        assert_eq!(reported("oval 2\n"), unsupported(0, 6));
        assert_eq!(reported("oval 99999999999999999999\n"), unsupported(0, 25));
    }

    #[test]
    fn a_marker_with_no_integer_names_no_version() {
        assert_eq!(reported("oval\n"), unsupported(0, 4));
        assert_eq!(reported("oval 1x\n"), unsupported(0, 4));
        assert_eq!(reported("oval \"1\"\n"), unsupported(0, 8));
    }

    #[test]
    fn an_unsupported_version_spans_the_marker_up_to_the_end_of_its_line() {
        assert_eq!(reported("oval 2 ~  # c\n"), unsupported(0, 6));
    }

    #[test]
    fn a_missing_marker_is_reported_at_the_first_token_the_parser_does_not_skip() {
        assert_eq!(reported("guide \"G\"\n"), unsupported(0, 0));
        assert_eq!(reported("\n# c\n~\n  guide \"G\"\n"), unsupported(9, 9));
    }

    #[test]
    fn a_missing_marker_in_a_document_without_content_is_reported_at_the_end() {
        assert_eq!(reported(""), unsupported(0, 0));
        assert_eq!(reported("# c\n"), unsupported(4, 4));
    }

    #[test]
    fn a_misplaced_marker_is_not_the_documents_marker() {
        assert_eq!(reported("guide \"G\"\noval 1\n"), unsupported(0, 0));
    }

    #[test]
    fn an_unsupported_version_is_reported_alone() {
        assert_eq!(reported("oval 2\nguide 5 ~\n"), unsupported(0, 6));
    }
}
