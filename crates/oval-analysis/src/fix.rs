use oval_core::index::Index;
use oval_core::{Diagnostic, DiagnosticCode, ValidationRule};
use oval_syntax::ast::{AstNode, Document};
use oval_syntax::{TextRange, TextSize};
use serde::Serialize;

use crate::symbols::{serialize_range, serialize_ranges};

/// One replacement in the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Edit {
    /// The range to replace, empty for an insertion.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// The text to put there.
    pub text: String,
    /// Ranges within `text` that the author should replace, in the order an editor visits them.
    /// Empty when the text is final.
    #[serde(
        serialize_with = "serialize_ranges",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub placeholders: Vec<TextRange>,
}

impl Edit {
    fn new(range: TextRange, text: String) -> Self {
        Self {
            range,
            text,
            placeholders: Vec::new(),
        }
    }
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
/// `missing-guide-field` is added after the version marker, with its value left as a placeholder
/// for the author to fill in. A `missing-recommendation` becomes
/// `recommend no-position` at the end of the contest or measure. An `unused-declaration` at the
/// top level is removed, unless it declares a party inline that something else names. Inserted
/// lines end with the document's own line terminator. Fixes are in source order, and a diagnostic
/// with no repair has no fix.
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
        ValidationRule::MissingGuideField => missing_guide_field(source, document, diagnostic)?,
        ValidationRule::MissingRecommendation => missing_recommendation(source, index, range)?,
        ValidationRule::UnusedDeclaration => unused_declaration(document, index, range)?,
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
        vec![Edit::new(scheme, "https://".to_owned())],
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
        vec![Edit::new(reference.name_range, nearest)],
    ))
}

fn missing_guide_field(
    source: &str,
    document: &Document,
    diagnostic: &Diagnostic,
) -> Option<(String, Vec<Edit>)> {
    let name = diagnostic
        .message
        .split('`')
        .nth(1)
        .filter(|name| ["guide", "election", "date"].contains(name))?;
    let (line, placeholder) = match name {
        "date" => ("date YYYY-MM-DD".to_owned(), 5..15),
        _ => (format!("{name} \"\""), name.len() + 2..name.len() + 2),
    };
    let terminator = terminator(source);
    let (text, before_line) = if document.version().is_some() {
        (format!("{terminator}{line}"), terminator.len())
    } else {
        (format!("{line}{terminator}"), 0)
    };
    let placeholder = TextRange::new(
        TextSize::try_from(before_line + placeholder.start).ok()?,
        TextSize::try_from(before_line + placeholder.end).ok()?,
    );
    Some((
        format!("Add `{name}`"),
        vec![Edit {
            range: TextRange::empty(diagnostic.range.start()),
            text,
            placeholders: vec![placeholder],
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
    let terminator = terminator(source);
    let needs_terminator =
        usize::from(end) > 0 && !source[..usize::from(end)].ends_with(['\n', '\r']);
    let text = format!(
        "{}{}recommend no-position{terminator}",
        if needs_terminator { terminator } else { "" },
        " ".repeat(indent)
    );
    Some((
        "Add `recommend no-position`".to_owned(),
        vec![Edit::new(TextRange::empty(end), text)],
    ))
}

fn unused_declaration(
    document: &Document,
    index: &Index,
    range: TextRange,
) -> Option<(String, Vec<Edit>)> {
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
    let declares_something_used_elsewhere = index
        .references
        .iter()
        .filter(|reference| !construct.contains_range(reference.range))
        .filter_map(|reference| index.resolve(&reference.name))
        .any(|declaration| construct.contains_range(declaration.range));
    if declares_something_used_elsewhere {
        return None;
    }
    Some((
        format!("Remove `@{}`", name.unwrap_or_default()),
        vec![Edit::new(construct, String::new())],
    ))
}

fn line_indent(source: &str, at: TextSize) -> usize {
    let at = usize::from(at);
    let line_start = source[..at].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    source[line_start..at]
        .chars()
        .take_while(|c| *c == ' ')
        .count()
}

fn terminator(source: &str) -> &'static str {
    match source.find(['\n', '\r']) {
        Some(at) if source[at..].starts_with("\r\n") => "\r\n",
        Some(at) if source[at..].starts_with('\r') => "\r",
        _ => "\n",
    }
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
