use oval_syntax::TextRange;
use oval_syntax::ast::AstNode;

use super::report;
use crate::index::Index;
use crate::{Diagnostic, ValidationRule};

/// Checks that every contest and measure has exactly one recommendation.
pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        let header = contest.header_line_range();
        let ranges: Vec<TextRange> = contest.recommendations().map(|r| r.range()).collect();
        exactly_one("contest", header, &ranges, out);
    }
    for measure in &index.measures {
        let header = measure.header_line_range();
        let ranges: Vec<TextRange> = measure.recommendations().map(|r| r.range()).collect();
        exactly_one("measure", header, &ranges, out);
    }
}

fn exactly_one(noun: &str, header: TextRange, ranges: &[TextRange], out: &mut Vec<Diagnostic>) {
    if ranges.is_empty() {
        out.push(report(
            ValidationRule::MissingRecommendation,
            header,
            format!(
                "this {noun} has no recommendation; write `recommend no-position` to take none"
            ),
        ));
    }
    for extra in ranges.iter().skip(1) {
        out.push(report(
            ValidationRule::RepeatedRecommendation,
            *extra,
            format!("this {noun} already has a recommendation"),
        ));
    }
}
