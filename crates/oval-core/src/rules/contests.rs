use oval_syntax::ast::{AstNode, Contest, ContestKind};
use oval_syntax::{SyntaxKind, TextRange};

use super::error;
use crate::index::Index;
use crate::{Diagnostic, Rule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        kind_arguments(contest, out);
    }
}

fn kind_arguments(contest: &Contest, out: &mut Vec<Diagnostic>) {
    let Some(kind) = contest.kind() else {
        return;
    };
    let range = kind_range(contest);
    match kind {
        ContestKind::Multiple { seats: Some(seats) } if seats < 2 => out.push(error(
            Rule::ContestKindArguments,
            range,
            "`multiple` needs at least 2 seats; a one-seat contest is `single`".to_owned(),
        )),
        ContestKind::Ranked { max: Some(0) } => out.push(error(
            Rule::ContestKindArguments,
            range,
            "`max` must be at least 1".to_owned(),
        )),
        ContestKind::Retention => {
            let campaigns = contest.campaigns().count();
            if campaigns != 1 {
                out.push(error(
                    Rule::ContestKindArguments,
                    range,
                    format!("a retention contest has exactly one campaign, not {campaigns}"),
                ));
            }
        }
        _ => {}
    }
}

fn kind_range(contest: &Contest) -> TextRange {
    contest
        .syntax()
        .children()
        .find(|n| n.kind() == SyntaxKind::ContestKind)
        .map_or_else(|| contest.range(), |n| n.text_range())
}
