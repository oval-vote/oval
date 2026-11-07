use oval_syntax::ast::{AstNode, Contest, ContestKind};
use oval_syntax::{SyntaxKind, TextRange};

use super::report;
use crate::index::Index;
use crate::{Diagnostic, ValidationRule};

/// Checks the contest kinds. A `multiple` count is at least 2, a `ranked` limit is at least 2, and a `retention` contest has exactly one campaign.
pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for contest in &index.contests {
        kind_arguments(contest, out);
    }
}

fn kind_arguments(contest: &Contest, out: &mut Vec<Diagnostic>) {
    let Some(kind) = contest.kind() else {
        return;
    };
    let argument = contest
        .kind_argument_range()
        .unwrap_or_else(|| kind_range(contest));
    match kind {
        ContestKind::Multiple { seats: Some(seats) } if seats < 2 => out.push(report(
            ValidationRule::InvalidMultipleCount,
            argument,
            "`multiple` needs at least 2 seats; a one-seat contest is `single`".to_owned(),
        )),
        ContestKind::Ranked { max: Some(0) } => out.push(report(
            ValidationRule::InvalidRankedMax,
            argument,
            "`max` must be at least 1".to_owned(),
        )),
        ContestKind::Retention => {
            let campaigns: Vec<_> = contest.campaigns().collect();
            if campaigns.is_empty() && contest.has_block() {
                out.push(report(
                    ValidationRule::RetentionCampaignCount,
                    contest.header_line_range(),
                    "a retention contest has exactly one campaign, and this one has none"
                        .to_owned(),
                ));
            }
            for extra in campaigns.iter().skip(1) {
                out.push(report(
                    ValidationRule::RetentionCampaignCount,
                    extra.range(),
                    "a retention contest has exactly one campaign".to_owned(),
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
        .map_or_else(|| contest.header_line_range(), |n| n.text_range())
}
