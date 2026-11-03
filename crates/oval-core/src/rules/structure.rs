use std::collections::HashSet;

use oval_syntax::ast::{AstNode, Ballot, BallotItem, ContestKind, Document};

use super::{error, warning};
use crate::index::Index;
use crate::{Diagnostic, Rule};

pub(crate) fn check(document: &Document, index: &Index, out: &mut Vec<Diagnostic>) {
    sibling_titles(document.ballots(), out);
    for ballot in &index.ballots {
        let children = ballot.items().filter_map(|item| match item {
            BallotItem::Ballot(sub) => Some(sub),
            _ => None,
        });
        sibling_titles(children, out);
    }
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
        for campaign in contest.campaigns() {
            if campaign.members().next().is_none() {
                out.push(error(
                    Rule::BallotStructure,
                    campaign.range(),
                    "this campaign has no members".to_owned(),
                ));
            }
        }
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

fn sibling_titles(ballots: impl Iterator<Item = Ballot>, out: &mut Vec<Diagnostic>) {
    let mut seen: HashSet<String> = HashSet::new();
    for ballot in ballots {
        let Some(title) = ballot.title() else {
            continue;
        };
        if !seen.insert(title.clone()) {
            let range = ballot.id().map_or_else(|| ballot.range(), |id| id.range());
            out.push(warning(
                Rule::SuspiciousBallots,
                range,
                format!("another ballot at this level is also titled \"{title}\""),
            ));
        }
    }
}
