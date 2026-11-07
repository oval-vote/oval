use std::collections::{HashMap, HashSet};

use oval_syntax::TextRange;
use oval_syntax::ast::{AstNode, Document};

use super::report;
use crate::index::{EntityKind, Index};
use crate::{Diagnostic, ValidationRule};

pub(crate) fn check(document: &Document, index: &Index, out: &mut Vec<Diagnostic>) {
    unused(document, index, out);
    duplicate_party_names(document, index, out);
}

fn unused(document: &Document, index: &Index, out: &mut Vec<Diagnostic>) {
    let referenced: HashSet<(&str, EntityKind)> = index
        .references
        .iter()
        .map(|r| (r.name.as_str(), r.expected))
        .collect();
    for party in document.parties() {
        let Some(id) = party.id() else { continue };
        if !referenced.contains(&(id.name().as_str(), EntityKind::Party)) {
            out.push(report(
                ValidationRule::UnusedDeclaration,
                id.range(),
                format!(
                    "`@{}` is declared but no candidate belongs to it",
                    id.name()
                ),
            ));
        }
    }
    for candidate in document.candidates() {
        let Some(id) = candidate.id() else { continue };
        if !referenced.contains(&(id.name().as_str(), EntityKind::Candidate)) {
            out.push(report(
                ValidationRule::UnusedDeclaration,
                id.range(),
                format!("`@{}` is declared but is not in any campaign", id.name()),
            ));
        }
    }
}

fn duplicate_party_names(document: &Document, index: &Index, out: &mut Vec<Diagnostic>) {
    let declared = document
        .parties()
        .filter_map(|p| Some((p.name()?, p.id()?)))
        .chain(
            index
                .inline_parties
                .iter()
                .filter_map(|p| Some((p.name()?, p.id()?))),
        );
    let mut by_name: HashMap<String, (String, TextRange)> = HashMap::new();
    let mut found: Vec<(TextRange, String)> = Vec::new();
    for (name, id) in declared {
        match by_name.get(&name) {
            Some((first, _)) if *first != id.name() => found.push((
                id.range(),
                format!(
                    "`@{}` has the same name as `@{first}`, \"{name}\"",
                    id.name()
                ),
            )),
            Some(_) => {}
            None => {
                by_name.insert(name, (id.name(), id.range()));
            }
        }
    }
    found.sort_by_key(|(range, _)| range.start());
    for (range, message) in found {
        out.push(report(ValidationRule::DuplicatePartyName, range, message));
    }
}
