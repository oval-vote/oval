//! Canonical formatting. The spec defines the grammar; this crate defines
//! the style. Formatting never changes the data model: `load(format(x))`
//! equals `load(x)`, which the test suite asserts for every conformance case.
//!
//! The style:
//! - LF line endings, no trailing whitespace, one trailing newline.
//! - Header fields in spec order, then `x-` fields; one blank line after.
//! - One blank line before every heading; none between a heading and its
//!   description, kind line, and fields.
//! - Tags in spec order; fields in spec order; `x-` and unknown ones after.
//! - Candidate lines sorted by marker (ranks, `+`, `*`, `-`), with one blank
//!   line before the block. In a contest `vote:` follows the candidates.
//! - Prose, descriptions, notes, and field values rewrapped at the width
//!   (default 80), continuation lines indented two spaces.
//! - Comments stay with the item that follows them; comments inside a
//!   paragraph move to just before it.
//!
//! A file with syntax errors is not formatted: fix the errors first.

use oval_syntax::ast::{self, Marker};
use oval_syntax::kind::SyntaxKind::*;
use oval_syntax::{join_segments, parse, Diagnostic, SyntaxNode};

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Wrap prose at this many characters. Lines are never broken inside a
    /// word, so a single long word may exceed it.
    pub width: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { width: 80 }
    }
}

/// Format `text`. Returns the parser's diagnostics instead when the file has
/// syntax errors.
pub fn format(text: &str, opts: &Options) -> Result<String, Vec<Diagnostic>> {
    let parse = parse(text);
    if !parse.diagnostics.is_empty() {
        return Err(parse.diagnostics);
    }
    let root = ast::Root::cast(parse.syntax()).expect("root");
    let mut f = Fmt {
        out: String::new(),
        width: opts.width,
    };
    f.root(&root);
    Ok(f.out)
}

/// Canonical field order per context. Keys not listed (unknown, `x-`) follow
/// in source order.
const HEADER_ORDER: &[&str] = &["name", "date", "author", "updated", "language"];
const CONTEST_ORDER: &[&str] = &["party", "note"]; // `vote` goes after the candidates
const MEASURE_ORDER: &[&str] = &[
    "placed by",
    "yes",
    "no",
    "threshold",
    "supporters",
    "opponents",
    "note",
    "vote",
];
const RETENTION_ORDER: &[&str] = &["designation", "note", "vote"];

/// Greedy word wrap. `first` prefixes the first line, `rest` the others.
pub fn wrap(text: &str, width: usize, first: &str, rest: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::from(first);
    let mut lead = first.len();
    for word in text.split_whitespace() {
        if line.len() > lead && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::replace(&mut line, String::from(rest)));
            lead = rest.len();
        }
        if line.len() > lead {
            line.push(' ');
        }
        line.push_str(word);
    }
    if line.len() > lead || lines.is_empty() {
        lines.push(line);
    }
    lines
}

/// A section body item with the comments that precede it.
struct Item<T> {
    comments: Vec<String>,
    node: T,
}

struct Fmt {
    out: String,
    width: usize,
}

impl Fmt {
    fn line(&mut self, s: &str) {
        self.out.push_str(s.trim_end());
        self.out.push('\n');
    }

    fn blank(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with("\n\n") {
            self.out.push('\n');
        }
    }

    fn comments(&mut self, comments: &[String]) {
        for c in comments {
            self.line(c);
        }
    }

    fn root(&mut self, root: &ast::Root) {
        let version = root.magic().and_then(|m| m.version()).unwrap_or(1);
        self.line(&format!("oval {version}"));
        if let Some(header) = root.header() {
            let (fields, trailing) = collect(header.syntax(), ast::Field::cast);
            let fields = ordered(fields, |f| key_rank(f, HEADER_ORDER));
            for f in &fields {
                self.comments(&f.comments);
                self.field(&f.node);
            }
            self.comments(&trailing);
        }
        for s in root.sections() {
            self.section(&s, &[]);
        }
    }

    fn section(&mut self, s: &ast::Section, leading: &[String]) {
        self.blank();
        self.comments(leading);
        if let Some(h) = s.heading() {
            let mut line = "#".repeat(h.level());
            line.push(' ');
            line.push_str(h.name().map(|n| n.text).unwrap_or_default().trim());
            if let Some(id) = h.id() {
                line.push_str(" @");
                line.push_str(id.text());
            }
            self.line(&line);
        }
        let kind = s.kind();
        let field_order: &[&str] = match kind.as_deref() {
            Some("contest") => CONTEST_ORDER,
            Some("measure") => MEASURE_ORDER,
            Some("retention") => RETENTION_ORDER,
            _ => &[],
        };

        // Gather the body in source order, attaching comments to what follows.
        let mut pending: Vec<String> = Vec::new();
        let mut description: Option<Item<ast::Description>> = None;
        let mut kind_line: Option<Item<ast::KindLine>> = None;
        let mut fields: Vec<Item<ast::Field>> = Vec::new();
        let mut candidates: Vec<Item<ast::Candidate>> = Vec::new();
        let mut paragraphs: Vec<Item<ast::Paragraph>> = Vec::new();
        let mut children: Vec<Item<ast::Section>> = Vec::new();
        for c in s.syntax().children() {
            let comments = std::mem::take(&mut pending);
            match c.kind() {
                COMMENT => pending.push(c.text().to_string().trim_end().to_string()),
                DESCRIPTION => {
                    description = ast::Description::cast(c).map(|node| Item { comments, node })
                }
                KIND_LINE => kind_line = ast::KindLine::cast(c).map(|node| Item { comments, node }),
                FIELD => fields.extend(ast::Field::cast(c).map(|node| Item { comments, node })),
                CANDIDATE => {
                    candidates.extend(ast::Candidate::cast(c).map(|node| Item { comments, node }))
                }
                PARAGRAPH => {
                    paragraphs.extend(ast::Paragraph::cast(c).map(|node| Item { comments, node }))
                }
                SECTION => {
                    children.extend(ast::Section::cast(c).map(|node| Item { comments, node }))
                }
                _ => pending = comments,
            }
        }
        let trailing = pending;

        if let Some(d) = &description {
            self.comments(&d.comments);
            let text = join_segments(&d.node.segments());
            for l in wrap(&text, self.width, "> ", "> ") {
                self.line(&l);
            }
        }
        if let Some(k) = &kind_line {
            self.comments(&k.comments);
            self.kind_line(&k.node, kind.as_deref());
        }
        let is_contest = kind.as_deref() == Some("contest");
        let (vote, fields): (Vec<_>, Vec<_>) = fields
            .into_iter()
            .partition(|f| is_contest && f.node.key().is_some_and(|k| k.text == "vote"));
        let fields = ordered(fields, |f| key_rank(f, field_order));
        for f in &fields {
            self.comments(&f.comments);
            self.field(&f.node);
        }
        if !candidates.is_empty() {
            self.blank();
            let candidates = ordered(candidates, |c| match c.marker() {
                Some(Marker::Rank(n)) => (0, n),
                Some(Marker::Pick) => (1, 0),
                Some(Marker::Neutral) | None => (2, 0),
                Some(Marker::Avoid) => (3, 0),
            });
            for c in &candidates {
                self.comments(&c.comments);
                self.candidate(&c.node);
            }
        }
        if !vote.is_empty() {
            self.blank();
            for f in &vote {
                self.comments(&f.comments);
                self.field(&f.node);
            }
        }
        for p in &paragraphs {
            self.blank();
            self.comments(&p.comments);
            self.paragraph(&p.node);
        }
        self.comments(&trailing);
        for c in &children {
            self.section(&c.node, &c.comments);
        }
    }

    fn kind_line(&mut self, k: &ast::KindLine, kind: Option<&str>) {
        let order: &[&str] = match kind {
            Some("contest") => &[
                "partisan",
                "ranked",
                "seats",
                "rank up to",
                "write-ins",
                "unverified",
            ],
            _ => &["unverified"],
        };
        let rank = |t: &str| {
            let word = if t.ends_with(" seats") {
                "seats"
            } else if t.starts_with("rank up to ") {
                "rank up to"
            } else {
                t
            };
            order.iter().position(|o| *o == word).unwrap_or(order.len())
        };
        let mut tags: Vec<String> = k.tags().into_iter().map(|t| t.text).collect();
        tags.sort_by_key(|t| rank(t));
        let mut line = k.keyword().unwrap_or_default();
        for t in tags {
            line.push_str(", ");
            line.push_str(&t);
        }
        self.line(&line);
    }

    fn field(&mut self, f: &ast::Field) {
        let key = f.key().map(|k| k.text).unwrap_or_default();
        let value = join_segments(&f.segments());
        if value.is_empty() {
            self.line(&format!("{key}:"));
            return;
        }
        for l in wrap(&value, self.width, &format!("{key}: "), "  ") {
            self.line(&l);
        }
    }

    fn candidate(&mut self, c: &ast::Candidate) {
        let mut line = match c.marker() {
            Some(Marker::Pick) => "+".to_string(),
            Some(Marker::Avoid) => "-".to_string(),
            Some(Marker::Neutral) | None => "*".to_string(),
            Some(Marker::Rank(n)) => format!("{n}."),
        };
        line.push(' ');
        line.push_str(&c.name().map(|n| n.text).unwrap_or_default());
        let labels: Vec<String> = c.labels().into_iter().map(|l| l.text).collect();
        if !labels.is_empty() {
            line.push_str(" (");
            line.push_str(&labels.join(", "));
            line.push(')');
        }
        if let Some(d) = c.designation() {
            line.push_str(": ");
            line.push_str(&d.text);
        }
        if let Some(id) = c.id() {
            line.push_str(" @");
            line.push_str(id.text());
        }
        self.line(&line);
        let note = join_segments(&c.note());
        if !note.is_empty() {
            for l in wrap(&note, self.width, "  ", "  ") {
                self.line(&l);
            }
        }
    }

    fn paragraph(&mut self, p: &ast::Paragraph) {
        let text = join_segments(&p.segments());
        for (i, l) in wrap(&text, self.width, "", "").into_iter().enumerate() {
            // A wrapped line must not become a heading or a comment.
            if i > 0 && (l.starts_with('#') || l.starts_with("//")) {
                self.line(&format!("\\{l}"));
            } else {
                self.line(&l);
            }
        }
    }
}

/// Split a node's children into cast items with their leading comments, plus
/// the comments after the last item.
fn collect<T>(node: &SyntaxNode, cast: fn(SyntaxNode) -> Option<T>) -> (Vec<Item<T>>, Vec<String>) {
    let mut items = Vec::new();
    let mut pending = Vec::new();
    for c in node.children() {
        if c.kind() == COMMENT {
            pending.push(c.text().to_string().trim_end().to_string());
        } else if let Some(node) = cast(c) {
            items.push(Item {
                comments: std::mem::take(&mut pending),
                node,
            });
        }
    }
    (items, pending)
}

fn ordered<T, K: Ord>(items: Vec<Item<T>>, key: impl Fn(&T) -> K) -> Vec<Item<T>> {
    let mut items = items;
    items.sort_by_key(|i| key(&i.node)); // stable: ties keep source order
    items
}

fn key_rank(f: &ast::Field, order: &[&str]) -> usize {
    let key = f.key().map(|k| k.text).unwrap_or_default();
    order.iter().position(|k| *k == key).unwrap_or(order.len())
}
