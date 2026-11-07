use std::collections::HashSet;

use oval_syntax::TextRange;
use oval_syntax::ast::{AstNode, Ballot, BallotItem, ContestKind, Document};

use super::report;
use crate::index::Index;
use crate::{Diagnostic, ValidationRule};

pub(crate) fn check(document: &Document, index: &Index, out: &mut Vec<Diagnostic>) {
    if document.ballots().next().is_none() {
        out.push(report(
            ValidationRule::EmptyDocument,
            TextRange::empty(document.range().end()),
            "the document has no ballot".to_owned(),
        ));
    }
    sibling_titles(document.ballots(), out);
    for ballot in &index.ballots {
        let children = ballot.items().filter_map(|item| match item {
            BallotItem::Ballot(sub) => Some(sub),
            _ => None,
        });
        sibling_titles(children, out);
    }
    for ballot in &index.ballots {
        if ballot.has_block() && ballot.items().next().is_none() {
            out.push(report(
                ValidationRule::EmptyBallot,
                ballot.header_line_range(),
                "this ballot has no contests, measures, sections, or sub-ballots".to_owned(),
            ));
        }
    }
    for section in &index.sections {
        if section.has_block() && section.items().next().is_none() {
            out.push(report(
                ValidationRule::EmptySection,
                section.header_line_range(),
                "this section has no contests or measures".to_owned(),
            ));
        }
    }
    for contest in &index.contests {
        for campaign in contest.campaigns() {
            if campaign.has_block() && campaign.members().next().is_none() {
                out.push(report(
                    ValidationRule::CampaignWithoutMembers,
                    campaign.header_line_range(),
                    "this campaign has no members".to_owned(),
                ));
            }
        }
        if contest.has_block()
            && contest.kind() != Some(ContestKind::Retention)
            && contest.campaigns().next().is_none()
        {
            out.push(report(
                ValidationRule::ContestWithoutCampaigns,
                contest.header_line_range(),
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
            let range = ballot
                .title_range()
                .unwrap_or_else(|| ballot.header_line_range());
            out.push(report(
                ValidationRule::DuplicateBallotTitle,
                range,
                format!("another ballot at this level is also titled \"{title}\""),
            ));
        }
    }
}
