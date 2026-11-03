use oval_syntax::ast::{AstNode, ContestKind};

use super::error;
use crate::index::Index;
use crate::{Diagnostic, Rule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for ballot in &index.ballots {
        if ballot.items().next().is_none() {
            let range = ballot.id().map_or_else(|| ballot.range(), |id| id.range());
            out.push(error(
                Rule::BallotStructure,
                range,
                "this ballot has no contests, measures, sections, or sub-ballots".to_owned(),
            ));
        }
    }
    for section in &index.sections {
        if section.items().next().is_none() {
            out.push(error(
                Rule::BallotStructure,
                section.range(),
                "this section has no contests or measures".to_owned(),
            ));
        }
    }
    for contest in &index.contests {
        if contest.kind() != Some(ContestKind::Retention) && contest.campaigns().next().is_none() {
            let range = contest
                .id()
                .map_or_else(|| contest.range(), |id| id.range());
            out.push(error(
                Rule::BallotStructure,
                range,
                "this contest has no campaigns".to_owned(),
            ));
        }
    }
}
