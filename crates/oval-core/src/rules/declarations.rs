use std::collections::{HashMap, HashSet};

use oval_syntax::TextRange;
use oval_syntax::ast::{AstNode, Document};

use super::report;
use crate::index::{EntityKind, Index, ReferenceSite};
use crate::{Diagnostic, ValidationRule};

/// Checks the declarations. Two parties do not share a name, and every top-level declaration is used.
pub(crate) fn check(document: &Document, index: &Index, out: &mut Vec<Diagnostic>) {
    unused(document, index, out);
    duplicate_party_names(document, index, out);
}

fn unused(document: &Document, index: &Index, out: &mut Vec<Diagnostic>) {
    let referenced: HashSet<(&str, EntityKind)> = index
        .references
        .iter()
        .filter(|r| r.site != ReferenceSite::Stance)
        .map(|r| (r.name.as_str(), r.expected))
        .collect();
    for party in document.parties() {
        let Some(id) = party.id() else { continue };
        if !referenced.contains(&(id.name().as_str(), EntityKind::Party)) {
            out.push(report(
                ValidationRule::UnusedDeclaration,
                party.header_line_range(),
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
                candidate.header_line_range(),
                format!("`@{}` is declared but is not in any campaign", id.name()),
            ));
        }
    }
}

fn duplicate_party_names(document: &Document, index: &Index, out: &mut Vec<Diagnostic>) {
    let declared = document
        .parties()
        .filter_map(|p| Some((p.name()?, p.name_range()?, p.id()?)))
        .chain(
            index
                .inline_parties
                .iter()
                .filter_map(|p| Some((p.name()?, p.name_range()?, p.id()?))),
        );
    let mut by_name: HashMap<String, String> = HashMap::new();
    let mut found: Vec<(TextRange, String)> = Vec::new();
    for (name, range, id) in declared {
        match by_name.get(&name) {
            Some(first) if *first != id.name() => found.push((
                range,
                format!(
                    "`@{}` has the same name as `@{first}`, \"{name}\"",
                    id.name()
                ),
            )),
            Some(_) => {}
            None => {
                by_name.insert(name, id.name());
            }
        }
    }
    found.sort_by_key(|(range, _)| range.start());
    for (range, message) in found {
        out.push(report(ValidationRule::DuplicatePartyName, range, message));
    }
}
