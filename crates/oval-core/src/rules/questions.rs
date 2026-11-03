use oval_syntax::TextRange;
use oval_syntax::ast::AstNode;

use super::error;
use crate::index::Index;
use crate::{Diagnostic, Rule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        let header = contest
            .id()
            .map_or_else(|| contest.range(), |id| id.range());
        let ranges: Vec<TextRange> = contest.recommendations().map(|r| r.range()).collect();
        exactly_one("contest", header, &ranges, out);
    }
    for measure in &index.measures {
        let header = measure
            .id()
            .map_or_else(|| measure.range(), |id| id.range());
        let ranges: Vec<TextRange> = measure.recommendations().map(|r| r.range()).collect();
        exactly_one("measure", header, &ranges, out);
    }
}

fn exactly_one(noun: &str, header: TextRange, ranges: &[TextRange], out: &mut Vec<Diagnostic>) {
    if ranges.is_empty() {
        out.push(error(
            Rule::EveryQuestionHasARecommendation,
            header,
            format!(
                "this {noun} has no recommendation; write `recommend no-position` to take none"
            ),
        ));
    }
    for extra in ranges.iter().skip(1) {
        out.push(error(
            Rule::EveryQuestionHasARecommendation,
            *extra,
            format!("this {noun} already has a recommendation"),
        ));
    }
}
