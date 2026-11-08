use oval_core::index::{EntityKind, Index};
use oval_syntax::ast::{AstNode, Document};
use oval_syntax::parser::{SyntaxError, parse};
use oval_syntax::{SyntaxKind, TextRange, TextSize};
use serde::Serialize;

use crate::names::display_names;
use crate::symbols::{SymbolKind, serialize_range};

const PLACEHOLDER: &str = "zqovalcompletionzq";

/// The kind of thing a completion inserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompletionKind {
    /// A keyword of the language, such as `contest` or `single`.
    Keyword,
    /// The identifier of a party.
    Party,
    /// The identifier of a candidate.
    Candidate,
    /// The identifier of a ballot.
    Ballot,
    /// The identifier of a contest.
    Contest,
    /// The identifier of a measure.
    Measure,
}

impl From<SymbolKind> for CompletionKind {
    fn from(kind: SymbolKind) -> Self {
        match kind {
            SymbolKind::Party => Self::Party,
            SymbolKind::Candidate => Self::Candidate,
            SymbolKind::Ballot => Self::Ballot,
            SymbolKind::Contest => Self::Contest,
            SymbolKind::Measure => Self::Measure,
        }
    }
}

/// One thing the grammar accepts at a position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completion {
    /// The text to insert, which is a keyword or an identifier with its `@`.
    pub label: String,
    /// The kind of thing the label is.
    pub kind: CompletionKind,
    /// The name the identifier declares, such as a candidate's name, for an identifier.
    pub detail: Option<String>,
    /// The range the label replaces, which covers the partial word before the position.
    #[serde(serialize_with = "serialize_range")]
    pub replace: TextRange,
}

/// Everything the grammar accepts at `offset` in `source`.
///
/// The position is treated as the end of whatever word it is in, so an editor passes the cursor
/// and gets every completion for the current line, keywords and identifiers alike. The editor
/// filters by the text in `replace`. Keywords are the ones the parser would accept there, so a
/// field already written in a block is not offered again, and identifiers are the declarations
/// of the kind the position names, so `support` offers candidates and `party` offers parties.
///
/// The result is empty inside a string, a comment, or a Markdown body, where the grammar accepts
/// anything, and where the position names a new identifier rather than an existing one.
pub fn completions(source: &str, offset: TextSize) -> Vec<Completion> {
    let offset = usize::from(offset).min(source.len());
    let offset = (0..=offset)
        .rev()
        .find(|at| source.is_char_boundary(*at))
        .unwrap_or(0);
    let replace = replace_range(source, offset);
    let start = usize::from(replace.start());
    let line_end = source[offset..]
        .find(['\n', '\r'])
        .map_or(source.len(), |at| offset + at);
    let probe = |insert: &str| format!("{}{insert}{}", &source[..start], &source[line_end..]);
    let at = replace.start();

    let probed = if source[start..offset].starts_with('@') {
        probe(&format!("@{PLACEHOLDER}"))
    } else {
        probe(PLACEHOLDER)
    };
    let Ok(parsed) = parse(&probed) else {
        return Vec::new();
    };
    let token = parsed
        .syntax()
        .token_at_offset(at)
        .right_biased()
        .map(|token| token.kind());
    if matches!(
        token,
        Some(SyntaxKind::StringLiteral | SyntaxKind::Comment | SyntaxKind::MarkdownBlock)
    ) {
        return Vec::new();
    }
    let Some(document) = Document::cast(parsed.syntax()) else {
        return Vec::new();
    };

    let expected: Vec<SyntaxKind> = parsed
        .errors()
        .iter()
        .find_map(|error| match error {
            SyntaxError::Parse(error) if error.range.contains(at) => Some(error.expected.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let mut found = Vec::new();
    if expected.is_empty() {
        let index = Index::build(&document);
        if let Some(kind) = reference_kind(&index) {
            found.extend(identifiers(&document, &index, kind, replace));
        }
        return found;
    }
    for kind in expected {
        if let Some(keyword) = kind.as_keyword() {
            found.push(Completion {
                label: keyword.text().to_owned(),
                kind: CompletionKind::Keyword,
                detail: None,
                replace,
            });
        } else if kind == SyntaxKind::At {
            let Ok(parsed) = parse(&probe(&format!("@{PLACEHOLDER}"))) else {
                continue;
            };
            let Some(document) = Document::cast(parsed.syntax()) else {
                continue;
            };
            let index = Index::build(&document);
            if let Some(kind) = reference_kind(&index) {
                found.extend(identifiers(&document, &index, kind, replace));
            }
        }
    }
    found
}

fn replace_range(source: &str, offset: usize) -> TextRange {
    let bytes = source.as_bytes();
    let mut start = offset;
    while start > 0
        && (bytes[start - 1].is_ascii_alphanumeric() || matches!(bytes[start - 1], b'-' | b'_'))
    {
        start -= 1;
    }
    if start > 0 && bytes[start - 1] == b'@' {
        start -= 1;
    }
    TextRange::new(text_size(start), text_size(offset))
}

fn reference_kind(index: &Index) -> Option<EntityKind> {
    index
        .references
        .iter()
        .find(|reference| reference.name == PLACEHOLDER)
        .map(|reference| reference.expected)
}

fn identifiers(
    document: &Document,
    index: &Index,
    kind: EntityKind,
    replace: TextRange,
) -> Vec<Completion> {
    let names = display_names(document, index);
    index
        .declarations
        .iter()
        .filter(|declaration| declaration.kind == kind && declaration.name != PLACEHOLDER)
        .map(|declaration| Completion {
            label: format!("@{}", declaration.name),
            kind: SymbolKind::from(declaration.kind).into(),
            detail: names.get(&declaration.name).cloned(),
            replace,
        })
        .collect()
}

fn text_size(offset: usize) -> TextSize {
    TextSize::try_from(offset).expect("the source is shorter than 4 GiB")
}
