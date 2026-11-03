use oval_syntax::ast::{AstNode, Document, VersionMarker};
use oval_syntax::{SPEC_VERSION, SpecVersion, SyntaxKind, TextRange};

use super::error;
use crate::{Diagnostic, Rule};

pub(crate) fn check(document: &Document, out: &mut Vec<Diagnostic>) -> bool {
    let Some(marker) = document.version() else {
        out.push(error(
            Rule::SupportedVersion,
            TextRange::empty(0.into()),
            format!(
                "the document has no version marker; its first line should be `oval {SPEC_VERSION}`"
            ),
        ));
        return true;
    };
    let Some(version) = marker.value() else {
        return true;
    };
    if supports(SPEC_VERSION, version) {
        return true;
    }
    out.push(error(
        Rule::SupportedVersion,
        literal_range(&marker),
        format!(
            "this document is written to Oval {version}, but this implementation reads Oval {SPEC_VERSION}"
        ),
    ));
    false
}

fn supports(implementation: SpecVersion, document: SpecVersion) -> bool {
    if implementation.major == 0 {
        document == implementation
    } else {
        document.major == implementation.major && document.minor <= implementation.minor
    }
}

fn literal_range(marker: &VersionMarker) -> TextRange {
    marker
        .syntax()
        .children_with_tokens()
        .filter_map(oval_syntax::SyntaxElement::into_token)
        .find(|token| token.kind() == SyntaxKind::VersionLiteral)
        .map_or_else(|| marker.range(), |token| token.text_range())
}

#[cfg(test)]
mod tests {
    use oval_syntax::SpecVersion;

    use super::supports;

    fn v(major: u32, minor: u32) -> SpecVersion {
        SpecVersion { major, minor }
    }

    #[test]
    fn draft_versions_match_exactly() {
        assert!(supports(v(0, 1), v(0, 1)));
        assert!(!supports(v(0, 2), v(0, 1)));
        assert!(!supports(v(0, 1), v(0, 2)));
        assert!(!supports(v(0, 1), v(1, 0)));
    }

    #[test]
    fn stable_versions_accept_older_minors_of_the_same_major() {
        assert!(supports(v(1, 3), v(1, 0)));
        assert!(supports(v(1, 3), v(1, 3)));
        assert!(!supports(v(1, 3), v(1, 4)));
        assert!(!supports(v(1, 3), v(0, 1)));
        assert!(!supports(v(1, 3), v(2, 0)));
    }
}
