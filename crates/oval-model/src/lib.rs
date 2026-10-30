//! The Oval Vote data model (spec section 5), built from a syntax tree with
//! the semantic rules of section 6 applied.

pub mod lower;
mod validate;

use oval_syntax::ast::Marker;
use oval_syntax::inline::Inline;
use oval_syntax::TextRange;
use serde::Serialize;
use std::collections::BTreeMap;

/// Where a node sits in the source. Skipped in JSON output; used by editors
/// and the linter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    /// The whole construct, including children.
    pub full: TextRange,
    /// The line that names it: the heading, or the candidate line.
    pub line: TextRange,
}

/// `x-` tags and fields, preserved but not interpreted (spec section 8).
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize)]
pub struct Extensions {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

impl Extensions {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.fields.is_empty()
    }
}

pub use lower::{load, Loaded};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Guide {
    pub version: u32,
    pub name: String,
    pub date: String,
    pub author: Author,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    pub language: String,
    #[serde(skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
    pub children: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Author {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Node {
    Section(Section),
    Contest(Contest),
    Measure(Measure),
    Retention(Retention),
}

impl Node {
    pub fn id(&self) -> &str {
        match self {
            Node::Section(s) => &s.id,
            Node::Contest(c) => &c.id,
            Node::Measure(m) => &m.id,
            Node::Retention(r) => &r.id,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Node::Section(s) => &s.name,
            Node::Contest(c) => &c.name,
            Node::Measure(m) => &m.name,
            Node::Retention(r) => &r.name,
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Node::Section(s) => s.span,
            Node::Contest(c) => c.span,
            Node::Measure(m) => m.span,
            Node::Retention(r) => r.span,
        }
    }
    pub fn children(&self) -> &[Node] {
        match self {
            Node::Section(s) => &s.children,
            _ => &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Section {
    #[serde(skip)]
    pub span: Span,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<Inline>>,
    pub region: bool,
    #[serde(skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
    pub children: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Method {
    Single,
    Multiple {
        seats: u32,
    },
    Ranked {
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Undecided,
    None,
    Decided,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Vote {
    Yes,
    No,
    Undecided,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Position {
    Pick,
    Rank { rank: u32 },
    Neutral,
    Avoid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Contest {
    #[serde(skip)]
    pub span: Span,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<Inline>>,
    pub partisan: bool,
    pub method: Method,
    pub write_ins: bool,
    pub unverified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Inline>>,
    pub candidates: Vec<Candidate>,
    pub decision: Decision,
    pub rationale: Vec<Vec<Inline>>,
    #[serde(skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Candidate {
    #[serde(skip)]
    pub span: Span,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    pub incumbent: bool,
    pub write_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Inline>>,
    pub position: Position,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Measure {
    #[serde(skip)]
    pub span: Span,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<Inline>>,
    pub unverified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<String>,
    pub supporters: Vec<String>,
    pub opponents: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Inline>>,
    pub vote: Vote,
    pub rationale: Vec<Vec<Inline>>,
    #[serde(skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Retention {
    #[serde(skip)]
    pub span: Span,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<Inline>>,
    pub unverified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Inline>>,
    pub vote: Vote,
    pub rationale: Vec<Vec<Inline>>,
    #[serde(skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
}

/// Derived identifier (spec 6.1): lowercase, diacritics removed, runs of
/// anything outside `a-z0-9` become `-`.
impl Position {
    /// The candidate-line marker this position came from.
    pub fn marker(self) -> Marker {
        match self {
            Position::Rank { rank } => Marker::Rank(rank),
            Position::Pick => Marker::Pick,
            Position::Neutral => Marker::Neutral,
            Position::Avoid => Marker::Avoid,
        }
    }
}

/// Canonical candidate order; see [`Marker::order_key`].
pub fn candidate_order(c: &Candidate) -> (u8, u32) {
    c.position.marker().order_key()
}

pub fn slug(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let mut out = String::new();
    let mut dash = false;
    for c in name.nfd() {
        if unicode_normalization::char::is_combining_mark(c) {
            continue;
        }
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}
