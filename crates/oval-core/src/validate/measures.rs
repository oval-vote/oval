use oval_syntax::ast::{AstNode, Document, Measure};

use super::{content_range, diagnostic, kept_nodes};
use crate::{Diagnostic, ValidationRule};

pub(super) fn unpaired_outcomes(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for measure in kept_nodes(document.syntax()).filter_map(Measure::cast) {
        let ((Some(unpaired), None) | (None, Some(unpaired))) = (measure.if_yes(), measure.if_no())
        else {
            continue;
        };
        if let Some(range) = content_range(unpaired.syntax()) {
            diagnostics.push(diagnostic(ValidationRule::UnpairedOutcome, range));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::validate::reported;

    #[test]
    fn a_measure_with_only_one_outcome_is_unpaired() {
        let text = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  measure @m \"M\"\n    if_yes:\n      Y.\n    recommend yes\n  measure @m2 \"M2\"\n    if_no:  # c\n      N.\n    recommend no\n  measure @m3 \"M3\"\n    if_yes:\n      Y.\n    if_no:\n      N.\n    recommend yes\n  measure @m4 \"M4\"\n    if_yes:\n      Y.\n    bogus if_no:\n      N.\n    recommend yes\n";
        assert_eq!(
            reported(text),
            [
                ("unpaired-outcome", 81, 98),
                ("unpaired-outcome", 139, 160),
                ("unpaired-outcome", 278, 295),
                ("unexpected-entry", 299, 321),
            ]
        );
    }
}
