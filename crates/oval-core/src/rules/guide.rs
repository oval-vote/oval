use oval_syntax::TextRange;
use oval_syntax::ast::{AstNode, Document};

use super::report;
use crate::{Diagnostic, ValidationRule};

/// Checks that the `guide`, `election`, and `date` fields are present.
pub(crate) fn check(document: &Document, out: &mut Vec<Diagnostic>) {
    let missing = [
        ("guide", document.guide().is_none()),
        ("election", document.election().is_none()),
        ("date", document.date().is_none()),
    ];
    let position = document
        .version()
        .map_or_else(|| 0.into(), |marker| marker.header_line_range().end());
    for (name, is_missing) in missing {
        if is_missing {
            out.push(report(
                ValidationRule::MissingGuideField,
                TextRange::empty(position),
                format!("the document has no `{name}` field"),
            ));
        }
    }
}
