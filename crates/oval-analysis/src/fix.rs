use oval_core::index::Index;
use oval_core::{Diagnostic, DiagnosticCode, ValidationRule};
use oval_syntax::ast::{AstNode, Document};
use oval_syntax::{TextRange, TextSize};
use serde::Serialize;

use crate::symbols::serialize_range;

/// One replacement in the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Edit {
    /// The range to replace, empty for an insertion.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// The text to put there.
    pub text: String,
}

/// A repair for one diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fix {
    /// The identifier of the diagnostic the fix repairs, such as `insecure-url`.
    pub code: &'static str,
    /// The range of that diagnostic, so an editor can pair the two.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// What the fix does, as an editor shows it.
    pub title: String,
    /// The edits to apply together.
    pub edits: Vec<Edit>,
}

/// Repairs for the diagnostics of `source` that have a mechanical one.
///
/// Five diagnostics qualify. An `insecure-url` becomes `https`. An `undeclared-reference` whose
/// name is close to a declared one of the kind the position accepts is changed to that name. A
/// `missing-guide-field` is added after the version marker. A `missing-recommendation` becomes
/// `recommend no-position` at the end of the contest or measure. An `unused-declaration` at the
/// top level is removed. Fixes are in source order, and a diagnostic with no repair has no fix.
pub fn fixes(source: &str) -> Vec<Fix> {
    let Ok(report) = oval_core::check(source) else {
        return Vec::new();
    };
    let Some(document) = Document::cast(report.parse().syntax()) else {
        return Vec::new();
    };
    let index = Index::build(&document);
    let mut found: Vec<Fix> = report
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| fix(source, &document, &index, diagnostic))
        .collect();
    found.sort_by_key(|fix| fix.range.start());
    found
}

fn fix(source: &str, document: &Document, index: &Index, diagnostic: &Diagnostic) -> Option<Fix> {
    let DiagnosticCode::Validation(rule) = diagnostic.code else {
        return None;
    };
    let range = diagnostic.range;
    let (title, edits) = match rule {
        ValidationRule::InsecureUrl => insecure_url(source, range)?,
        ValidationRule::UndeclaredReference => undeclared_reference(index, range)?,
        ValidationRule::MissingGuideField => missing_guide_field(document, diagnostic)?,
        ValidationRule::MissingRecommendation => missing_recommendation(source, index, range)?,
        ValidationRule::UnusedDeclaration => unused_declaration(document, range)?,
        _ => return None,
    };
    Some(Fix {
        code: diagnostic.code.identifier(),
        range,
        title,
        edits,
    })
}

fn insecure_url(source: &str, range: TextRange) -> Option<(String, Vec<Edit>)> {
    let scheme = TextRange::at(range.start(), TextSize::from(7));
    if source.get(std::ops::Range::<usize>::from(scheme)) != Some("http://") {
        return None;
    }
    Some((
        "Use https".to_owned(),
        vec![Edit {
            range: scheme,
            text: "https://".to_owned(),
        }],
    ))
}

fn undeclared_reference(index: &Index, range: TextRange) -> Option<(String, Vec<Edit>)> {
    let reference = index.references.iter().find(|r| r.range == range)?;
    let nearest = index
        .declarations
        .iter()
        .filter(|d| d.kind == reference.expected)
        .map(|d| (distance(&d.name, &reference.name), &d.name))
        .filter(|(distance, name)| *distance <= (name.len() / 3).max(2))
        .min_by_key(|(distance, name)| (*distance, name.len()))
        .map(|(_, name)| name.clone())?;
    Some((
        format!("Change to `@{nearest}`"),
        vec![Edit {
            range: reference.name_range,
            text: nearest,
        }],
    ))
}

fn missing_guide_field(
    document: &Document,
    diagnostic: &Diagnostic,
) -> Option<(String, Vec<Edit>)> {
    let name = diagnostic
        .message
        .split('`')
        .nth(1)
        .filter(|name| ["guide", "election", "date"].contains(name))?;
    let line = match name {
        "date" => "date 2026-01-01".to_owned(),
        _ => format!("{name} \"\""),
    };
    let text = if document.version().is_some() {
        format!("\n{line}")
    } else {
        format!("{line}\n")
    };
    Some((
        format!("Add `{line}`"),
        vec![Edit {
            range: TextRange::empty(diagnostic.range.start()),
            text,
        }],
    ))
}

fn missing_recommendation(
    source: &str,
    index: &Index,
    range: TextRange,
) -> Option<(String, Vec<Edit>)> {
    let (header, construct) = index
        .contests
        .iter()
        .map(|c| (c.header_line_range(), c.range()))
        .chain(
            index
                .measures
                .iter()
                .map(|m| (m.header_line_range(), m.range())),
        )
        .find(|(header, _)| *header == range)?;
    let indent = line_indent(source, header.start()) + 2;
    let end = construct.end();
    let needs_newline = usize::from(end) > 0 && !source[..usize::from(end)].ends_with('\n');
    let text = format!(
        "{}{}recommend no-position\n",
        if needs_newline { "\n" } else { "" },
        " ".repeat(indent)
    );
    Some((
        "Add `recommend no-position`".to_owned(),
        vec![Edit {
            range: TextRange::empty(end),
            text,
        }],
    ))
}

fn unused_declaration(document: &Document, range: TextRange) -> Option<(String, Vec<Edit>)> {
    let (name, construct) = document
        .parties()
        .map(|p| (p.id(), p.header_line_range(), p.range()))
        .chain(
            document
                .candidates()
                .map(|c| (c.id(), c.header_line_range(), c.range())),
        )
        .find(|(_, header, _)| *header == range)
        .map(|(id, _, construct)| (id.map(|id| id.name()), construct))?;
    Some((
        format!("Remove `@{}`", name.unwrap_or_default()),
        vec![Edit {
            range: construct,
            text: String::new(),
        }],
    ))
}

fn line_indent(source: &str, at: TextSize) -> usize {
    let at = usize::from(at);
    let line_start = source[..at].rfind('\n').map_or(0, |i| i + 1);
    source[line_start..at]
        .chars()
        .take_while(|c| *c == ' ')
        .count()
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current.push(
                (previous[j] + cost)
                    .min(previous[j + 1] + 1)
                    .min(current[j] + 1),
            );
        }
        previous = current;
    }
    previous[b.len()]
}
