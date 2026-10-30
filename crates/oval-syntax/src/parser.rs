//! Line-oriented parser. Each source line is classified by its first
//! characters, then emitted as tokens under the node the current context
//! calls for. Nothing is ever dropped: a line that cannot appear where it is
//! becomes an `ERROR` node wrapping its normal parse, with a diagnostic.

use crate::ast::SectionKind;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::kind::SyntaxKind::{self, *};
use crate::SyntaxNode;
use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

/// The result of parsing: a lossless tree and the diagnostics found while
/// building it. Semantic diagnostics (ids, decisions, counts) come from the
/// model crate, not from here.
#[derive(Clone, Debug)]
pub struct Parse {
    pub green: GreenNode,
    pub diagnostics: Vec<Diagnostic>,
}

impl Parse {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
}

pub fn parse(src: &str) -> Parse {
    let mut p = Parser {
        lines: split_lines(src),
        pos: 0,
        b: GreenNodeBuilder::new(),
        diags: Diagnostics::new(),
        open: Vec::new(),
    };
    p.file();
    Parse { green: p.b.finish(), diagnostics: p.diags.into_vec() }
}

#[derive(Clone, Copy)]
struct Line<'a> {
    start: usize,
    text: &'a str, // without the terminator
    nl: &'a str,   // "", "\n", or "\r\n"
}

impl Line<'_> {
    fn range(&self) -> TextRange {
        TextRange::new(ts(self.start), ts(self.start + self.text.len()))
    }
}

fn ts(n: usize) -> TextSize {
    TextSize::from(n as u32)
}

fn split_lines(src: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let (text_end, nl) = if i > start && bytes[i - 1] == b'\r' { (i - 1, "\r\n") } else { (i, "\n") };
            lines.push(Line { start, text: &src[start..text_end], nl });
            start = i + 1;
        }
        i += 1;
    }
    if start < src.len() || src.is_empty() {
        lines.push(Line { start, text: &src[start..], nl: "" });
    }
    lines
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LineKind {
    Blank,
    Comment,
    Heading,
    Tab,
    Continuation,
    Desc,
    Candidate,
    Field,
    Prose,
}

impl LineKind {
    /// Anything that is not trivia or a heading: a line that must belong to
    /// the current section's body.
    fn is_content(self) -> bool {
        !matches!(self, LineKind::Blank | LineKind::Comment | LineKind::Heading)
    }
}

fn classify(text: &str) -> LineKind {
    if text.trim().is_empty() {
        LineKind::Blank
    } else if text.starts_with("//") {
        LineKind::Comment
    } else if text.starts_with('#') {
        LineKind::Heading
    } else if text.starts_with('\t') {
        LineKind::Tab
    } else if text.starts_with(' ') {
        LineKind::Continuation
    } else if text.starts_with('>') {
        LineKind::Desc
    } else if marker_len(text).is_some() {
        LineKind::Candidate
    } else if field_key_len(text).is_some() {
        LineKind::Field
    } else {
        LineKind::Prose
    }
}

/// Length of a candidate marker (`+`, `-`, `*`, or digits and a dot) when the
/// line starts with one followed by a space.
fn marker_len(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    match b.first()? {
        b'+' | b'-' | b'*' => (b.get(1) == Some(&b' ')).then_some(1),
        b'0'..=b'9' => {
            let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
            (b.get(digits) == Some(&b'.') && b.get(digits + 1) == Some(&b' ')).then_some(digits + 1)
        }
        _ => None,
    }
}

/// Length of a field key when the line is `key:` or `key: value`, where key
/// is lowercase words separated by single spaces; a word may contain single
/// hyphens (`x-note`).
fn field_key_len(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    let mut i = 0;
    loop {
        let word = b[i..].iter().take_while(|c| c.is_ascii_lowercase()).count();
        if word == 0 {
            return None;
        }
        i += word;
        match b.get(i) {
            Some(b':') => return (matches!(b.get(i + 1), None | Some(b' '))).then_some(i),
            Some(b' ' | b'-') => i += 1,
            _ => return None,
        }
    }
}

/// The kind when the line is a kind line: a kind keyword followed by nothing
/// or by a comma.
fn kind_keyword(text: &str) -> Option<SectionKind> {
    let word = text.split([',', ' ']).next()?;
    let k = SectionKind::parse(word)?;
    let rest = text[k.as_str().len()..].trim_start();
    (rest.is_empty() || rest.starts_with(',')).then_some(k)
}

pub fn is_valid_id(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// What the parser knows about an open section's kind. The two recovery
/// states never reach the tree; only `Known` has a kind line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SecKind {
    /// No kind line (yet): holds only sections.
    Container,
    Known(SectionKind),
    /// Had content without a kind line; reported once, then parsed as a question.
    Unknown,
}

impl SecKind {
    fn is_question(self) -> bool {
        match self {
            SecKind::Container => false,
            SecKind::Known(k) => k.is_question(),
            SecKind::Unknown => true,
        }
    }
    fn allows_candidates(self) -> bool {
        matches!(self, SecKind::Known(SectionKind::Contest) | SecKind::Unknown)
    }
}

/// Where the parser is within a section's body. `AfterHeading` and
/// `AfterDescription` only occur while the kind is still `Container`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    AfterHeading,
    AfterDescription,
    Structure,
    Prose,
}

/// What `section_line` does with a line, decided once per line by `step`.
enum Action {
    Heading,
    Blank,
    Comment,
    Description,
    KindLine(SectionKind),
    Field,
    Candidate,
    Paragraph,
    /// Report `code`, then parse the construct normally inside an ERROR node.
    Misplaced {
        code: &'static str,
        msg: &'static str,
        construct: Construct,
    },
    /// Report `code` and keep the line as a bare ERROR node holding one TEXT token.
    Stray {
        code: &'static str,
        msg: &'static str,
    },
}

/// A construct that can be parsed in the wrong place.
#[derive(Clone, Copy)]
enum Construct {
    Field,
    Candidate,
    Description,
}

/// One line's outcome: the action, the section state once it is taken, and
/// whether this line is content in a section that never got a kind line
/// (reported once, before the action runs).
struct Step {
    action: Action,
    phase: Phase,
    kind: SecKind,
    missing_kind_line: bool,
}

/// The dispatch table for a line in a section body. Pure. `kind_line` is
/// `Some` when the line reads `kind` or `kind, tags`; it is only consulted
/// directly under the heading, so a later paragraph that starts with the
/// word `measure` is prose.
fn step(phase: Phase, kind: SecKind, line: LineKind, kind_line: Option<SectionKind>) -> Step {
    match phase {
        Phase::AfterHeading if line == LineKind::Desc => Step {
            action: Action::Description,
            phase: Phase::AfterDescription,
            kind,
            missing_kind_line: false,
        },
        Phase::AfterHeading | Phase::AfterDescription if line.is_content() => {
            if let Some(k) = kind_line {
                Step {
                    action: Action::KindLine(k),
                    phase: Phase::Structure,
                    kind: SecKind::Known(k),
                    missing_kind_line: false,
                }
            } else {
                // Content without a kind line: report once, become a question,
                // and dispatch this same line as structure.
                let missing = kind == SecKind::Container;
                let kind = if missing { SecKind::Unknown } else { kind };
                Step { missing_kind_line: missing, ..body(Phase::Structure, kind, line) }
            }
        }
        _ => body(phase, kind, line),
    }
}

/// Dispatch once the section's kind is settled, and for trivia in any phase.
fn body(phase: Phase, kind: SecKind, line: LineKind) -> Step {
    let prose = phase == Phase::Prose;
    let mut next = phase;
    let action = match line {
        LineKind::Heading => Action::Heading,
        LineKind::Blank => Action::Blank,
        LineKind::Comment => Action::Comment,
        _ if kind == SecKind::Known(SectionKind::Region) => Action::Stray {
            code: "content-in-container",
            msg: "a region holds sections, not fields, candidates, or rationale",
        },
        LineKind::Field if !prose => Action::Field,
        LineKind::Field => Action::Misplaced {
            code: "structure-after-rationale",
            msg: "fields must come before the rationale; move this line above the first paragraph",
            construct: Construct::Field,
        },
        LineKind::Candidate if prose => Action::Misplaced {
            code: "structure-after-rationale",
            msg: "candidate lines must come before the rationale; move this line above the first paragraph",
            construct: Construct::Candidate,
        },
        LineKind::Candidate if kind.allows_candidates() => Action::Candidate,
        LineKind::Candidate => Action::Misplaced {
            code: "candidate-in-non-contest",
            msg: "candidate lines are only allowed in a contest",
            construct: Construct::Candidate,
        },
        LineKind::Prose => {
            next = Phase::Prose;
            Action::Paragraph
        }
        LineKind::Desc => Action::Misplaced {
            code: "description-misplaced",
            msg: "a `>` description goes directly under the heading",
            construct: Construct::Description,
        },
        LineKind::Continuation => Action::Stray {
            code: "stray-continuation",
            msg: "an indented line continues the field or candidate line directly above it; nothing is above this one",
        },
        LineKind::Tab => Action::Stray { code: "tab-indent", msg: "lines are indented with spaces, not tabs" },
    };
    Step { action, phase: next, kind, missing_kind_line: false }
}

struct Open {
    level: usize,
    kind: SecKind,
    phase: Phase,
}

struct Parser<'a> {
    lines: Vec<Line<'a>>,
    pos: usize,
    b: GreenNodeBuilder<'static>,
    diags: Diagnostics,
    open: Vec<Open>,
}

impl<'a> Parser<'a> {
    // ----- helpers -----

    fn tok(&mut self, kind: SyntaxKind, text: &str) {
        if !text.is_empty() {
            self.b.token(kind.into(), text);
        }
    }

    fn nl(&mut self, line: Line) {
        self.tok(NEWLINE, line.nl);
    }

    fn error(&mut self, range: TextRange, code: &'static str, msg: impl Into<String>) {
        self.diags.error(range, code, msg);
    }

    fn peek(&self) -> Option<Line<'a>> {
        self.lines.get(self.pos).copied()
    }

    fn peek_kind(&self) -> Option<LineKind> {
        self.peek().map(|l| classify(l.text))
    }

    fn bump(&mut self) -> Line<'a> {
        let l = self.lines[self.pos];
        self.pos += 1;
        l
    }

    /// Emit `text` as WHITESPACE/TEXT/WHITESPACE with the surrounding spaces split out.
    fn trimmed(&mut self, kind: SyntaxKind, text: &str, wrap: Option<SyntaxKind>) {
        let lead = text.len() - text.trim_start().len();
        let inner = text.trim();
        let trail = text.len() - lead - inner.len();
        self.tok(WHITESPACE, &text[..lead]);
        if let Some(w) = wrap {
            self.b.start_node(w.into());
        }
        self.tok(kind, inner);
        if wrap.is_some() {
            self.b.finish_node();
        }
        self.tok(WHITESPACE, &text[text.len() - trail..]);
    }

    // ----- file -----

    fn file(&mut self) {
        self.b.start_node(ROOT.into());
        self.magic();
        self.b.start_node(HEADER.into());
        while let Some(line) = self.peek() {
            match classify(line.text) {
                LineKind::Heading => break,
                LineKind::Blank => self.blank(),
                LineKind::Comment => self.comment(),
                LineKind::Field => self.field(),
                _ => {
                    self.bump();
                    self.error(
                        line.range(),
                        "unexpected-line",
                        "only fields, comments, and blank lines may appear before the first heading",
                    );
                    self.error_line(line);
                }
            }
        }
        self.b.finish_node(); // HEADER
        while let Some(line) = self.peek() {
            self.section_line(line);
        }
        while self.open.pop().is_some() {
            self.b.finish_node();
        }
        self.b.finish_node(); // ROOT
    }

    fn magic(&mut self) {
        let Some(line) = self.peek() else { return };
        self.bump();
        let text = line.text.trim_end();
        let mut words = text.splitn(2, ' ');
        let kw = words.next().unwrap_or("");
        let ver = words.next().unwrap_or("");
        if kw == "oval" && !ver.is_empty() && ver.bytes().all(|b| b.is_ascii_digit()) {
            self.b.start_node(MAGIC.into());
            self.tok(KEYWORD, "oval");
            self.tok(WHITESPACE, " ");
            self.tok(NUMBER, ver);
            self.tok(WHITESPACE, &line.text[text.len()..]);
            self.nl(line);
            self.b.finish_node();
            if ver != "1" {
                self.error(
                    line.range(),
                    "unsupported-version",
                    format!("this parser implements Oval Vote version 1, the file declares version {ver}"),
                );
            }
        } else {
            self.error(line.range(), "bad-magic", "the first line must be `oval 1`");
            self.error_line(line);
        }
    }

    /// Wrap an already-consumed line as an ERROR node holding one TEXT token.
    fn error_line(&mut self, line: Line) {
        self.b.start_node(ERROR.into());
        self.tok(TEXT, line.text);
        self.nl(line);
        self.b.finish_node();
    }

    // ----- simple lines -----

    fn blank(&mut self) {
        let line = self.bump();
        self.b.start_node(BLANK.into());
        self.tok(WHITESPACE, line.text);
        self.nl(line);
        self.b.finish_node();
    }

    fn comment(&mut self) {
        let line = self.bump();
        self.b.start_node(COMMENT.into());
        self.tok(SLASHES, "//");
        self.tok(COMMENT_TEXT, &line.text[2..]);
        self.nl(line);
        self.b.finish_node();
    }

    // ----- sections -----

    fn section_line(&mut self, line: Line<'a>) {
        let line_kind = classify(line.text);
        if line_kind == LineKind::Heading {
            self.heading();
            return;
        }
        let (phase, kind) = {
            let cur = self.open.last().expect("a section is open after the header");
            (cur.phase, cur.kind)
        };
        let kind_line = if line_kind == LineKind::Prose { kind_keyword(line.text) } else { None };
        let step = step(phase, kind, line_kind, kind_line);
        if step.missing_kind_line {
            self.error(line.range(), "content-in-container", "this section has no kind line, so it can only contain other sections; add `contest`, `measure`, `retention`, or `region` under the heading, or remove this");
        }
        // The state is written before the action runs. That is safe because
        // no parse function below reads `open`.
        if let Some(cur) = self.open.last_mut() {
            cur.phase = step.phase;
            cur.kind = step.kind;
        }
        match step.action {
            Action::Heading => self.heading(),
            Action::Blank => self.blank(),
            Action::Comment => self.comment(),
            Action::Description => self.description(),
            Action::KindLine(k) => self.kind_line(k),
            Action::Field => self.field(),
            Action::Candidate => self.candidate(),
            Action::Paragraph => self.paragraph(),
            Action::Misplaced { code, msg, construct } => {
                self.error(line.range(), code, msg);
                self.wrap_error(match construct {
                    Construct::Field => Parser::field,
                    Construct::Candidate => Parser::candidate,
                    Construct::Description => Parser::description,
                });
            }
            Action::Stray { code, msg } => {
                self.bump();
                self.error(line.range(), code, msg);
                self.error_line(line);
            }
        }
    }

    fn wrap_error(&mut self, f: impl FnOnce(&mut Self)) {
        self.b.start_node(ERROR.into());
        f(self);
        self.b.finish_node();
    }

    fn heading(&mut self) {
        let line = self.bump();
        let level = line.text.bytes().take_while(|b| *b == b'#').count();
        // Close sections at this level or deeper.
        while self.open.last().is_some_and(|o| o.level >= level) {
            self.open.pop();
            self.b.finish_node();
        }
        let parent = self.open.last();
        match parent {
            None if level != 1 => {
                self.error(line.range(), "heading-level", "the first heading must be level 1 (`#`)");
            }
            Some(p) if level > p.level + 1 => self.error(
                line.range(),
                "heading-level",
                format!("heading level {level} under a level {} heading; levels may not skip", p.level),
            ),
            Some(p) if p.kind.is_question() => self.error(
                line.range(),
                "section-in-question",
                "a contest, measure, or retention cannot contain sections",
            ),
            _ => {}
        }
        if level > 6 {
            self.error(line.range(), "heading-level", "headings go at most six levels deep");
        }
        self.open.push(Open { level, kind: SecKind::Container, phase: Phase::AfterHeading });
        self.b.start_node(SECTION.into());
        self.b.start_node(HEADING.into());
        self.tok(HASHES, &line.text[..level]);
        let rest = &line.text[level..];
        let lead = rest.len() - rest.trim_start().len();
        self.tok(WHITESPACE, &rest[..lead]);
        let rest = &rest[lead..];
        let trimmed = rest.trim_end();
        let trail = &rest[trimmed.len()..];
        // Trailing `@id` token.
        let (name, id) = match trimmed.rsplit_once(char::is_whitespace) {
            Some((before, last)) if last.starts_with('@') => (before, Some(last)),
            _ if trimmed.starts_with('@') => ("", Some(trimmed)),
            _ => (trimmed, None),
        };
        if name.trim().is_empty() {
            self.error(line.range(), "empty-heading", "a heading needs a name");
        }
        if !name.is_empty() {
            self.trimmed(TEXT, name, Some(NAME));
        }
        if let Some(id) = id {
            let gap = &trimmed[name.len()..trimmed.len() - id.len()];
            self.tok(WHITESPACE, gap);
            self.tok(AT, "@");
            self.tok(IDENT, &id[1..]);
            if !is_valid_id(&id[1..]) {
                let start = line.start + level + lead + name.len() + gap.len();
                self.error(
                    TextRange::new(ts(start), ts(start + id.len())),
                    "bad-id",
                    "an id is letters, digits, `.`, `_`, and `-`, starting with a letter or digit",
                );
            }
        }
        self.tok(WHITESPACE, trail);
        self.nl(line);
        self.b.finish_node(); // HEADING
    }

    fn description(&mut self) {
        self.b.start_node(DESCRIPTION.into());
        while self.peek_kind() == Some(LineKind::Desc) {
            let line = self.bump();
            self.b.start_node(DESC_LINE.into());
            self.tok(GT, ">");
            self.trimmed(TEXT, &line.text[1..], None);
            self.nl(line);
            self.b.finish_node();
        }
        self.b.finish_node();
    }

    fn kind_line(&mut self, k: SectionKind) {
        let line = self.bump();
        let kw = k.as_str();
        self.b.start_node(KIND_LINE.into());
        self.tok(KEYWORD, kw);
        let mut rest = &line.text[kw.len()..];
        let mut offset = line.start + kw.len();
        loop {
            let lead = rest.len() - rest.trim_start().len();
            self.tok(WHITESPACE, &rest[..lead]);
            rest = &rest[lead..];
            offset += lead;
            if rest.is_empty() {
                break;
            }
            // rest starts with ',' by construction of kind_keyword.
            self.tok(COMMA, ",");
            rest = &rest[1..];
            offset += 1;
            let seg_len = rest.find(',').unwrap_or(rest.len());
            let seg = &rest[..seg_len];
            if seg.trim().is_empty() {
                self.error(
                    TextRange::new(ts(offset), ts(offset + seg_len)),
                    "empty-tag",
                    "empty tag after the comma",
                );
                self.tok(WHITESPACE, seg);
            } else {
                self.trimmed(TEXT, seg, Some(TAG));
            }
            rest = &rest[seg_len..];
            offset += seg_len;
        }
        self.nl(line);
        self.b.finish_node();
    }

    fn continuations(&mut self) {
        while self.peek_kind() == Some(LineKind::Continuation) {
            let line = self.bump();
            self.b.start_node(CONTINUATION.into());
            self.trimmed(TEXT, line.text, None);
            self.nl(line);
            self.b.finish_node();
        }
    }

    fn field(&mut self) {
        let line = self.bump();
        let key_len = field_key_len(line.text).unwrap();
        self.b.start_node(FIELD.into());
        self.b.start_node(KEY.into());
        self.tok(TEXT, &line.text[..key_len]);
        self.b.finish_node();
        self.tok(COLON, ":");
        let rest = &line.text[key_len + 1..];
        if rest.trim().is_empty() {
            self.tok(WHITESPACE, rest);
        } else {
            self.trimmed(TEXT, rest, Some(VALUE));
        }
        self.nl(line);
        self.continuations();
        self.b.finish_node();
    }

    fn candidate(&mut self) {
        let line = self.bump();
        let mlen = marker_len(line.text).unwrap();
        self.b.start_node(CANDIDATE.into());
        self.b.start_node(MARKER.into());
        match line.text.as_bytes()[0] {
            b'+' => self.tok(PLUS, "+"),
            b'-' => self.tok(MINUS, "-"),
            b'*' => self.tok(STAR, "*"),
            _ => {
                self.tok(NUMBER, &line.text[..mlen - 1]);
                self.tok(DOT, ".");
            }
        }
        self.b.finish_node();
        let rest = &line.text[mlen..];
        let lead = rest.len() - rest.trim_start().len();
        self.tok(WHITESPACE, &rest[..lead]);
        let rest = &rest[lead..];
        let body = rest.trim_end();
        let trail = &rest[body.len()..];

        // 1. trailing @id
        let (body, id) = match body.rsplit_once(' ') {
            Some((before, last)) if last.starts_with('@') => (before, Some((last, before.len()))),
            _ => (body, None),
        };
        // 2. designation after the first ": " (or a trailing ":")
        let (head, designation) = match body.find(':') {
            Some(i) if body[i + 1..].starts_with(' ') || body[i + 1..].is_empty() => {
                (&body[..i], Some(&body[i..]))
            }
            _ => (body, None),
        };
        // 3. labels in a trailing parenthesis
        let head_t = head.trim_end();
        let (name, labels) =
            match head_t.strip_suffix(')').and_then(|h| h.rfind(" (").map(|i| (&h[..i], &h[i + 2..]))) {
                Some((n, l)) => (n, Some(l)),
                None => (head_t, None),
            };
        if name.trim().is_empty() {
            self.error(line.range(), "empty-candidate", "a candidate line needs a name after the marker");
        } else {
            self.trimmed(TEXT, name, Some(NAME));
        }
        if let Some(l) = labels {
            self.tok(WHITESPACE, " ");
            self.tok(LPAREN, "(");
            self.b.start_node(LABELS.into());
            let mut first = true;
            for seg in l.split(',') {
                if !first {
                    self.tok(COMMA, ",");
                }
                first = false;
                if seg.trim().is_empty() {
                    self.error(line.range(), "empty-label", "empty label between commas");
                    self.tok(WHITESPACE, seg);
                } else {
                    self.trimmed(TEXT, seg, Some(LABEL));
                }
            }
            self.b.finish_node();
            self.tok(RPAREN, ")");
        }
        self.tok(WHITESPACE, &head[head_t.len()..]);
        if let Some(d) = designation {
            self.tok(COLON, ":");
            let v = &d[1..];
            if v.trim().is_empty() {
                self.error(
                    line.range(),
                    "empty-designation",
                    "a colon starts a designation; add one or remove the colon",
                );
                self.tok(WHITESPACE, v);
            } else {
                self.trimmed(TEXT, v, Some(DESIGNATION));
            }
        }
        if let Some((id, _)) = id {
            self.tok(WHITESPACE, " ");
            self.tok(AT, "@");
            self.tok(IDENT, &id[1..]);
            if !is_valid_id(&id[1..]) {
                self.error(
                    line.range(),
                    "bad-id",
                    "an id is letters, digits, `.`, `_`, and `-`, starting with a letter or digit",
                );
            }
        }
        self.tok(WHITESPACE, trail);
        self.nl(line);
        self.continuations();
        self.b.finish_node();
    }

    fn paragraph(&mut self) {
        self.b.start_node(PARAGRAPH.into());
        let first = self.bump();
        self.prose_line(first);
        loop {
            match self.peek_kind() {
                Some(LineKind::Comment) => {
                    // A comment inside a paragraph belongs to it only if prose follows.
                    let mut j = self.pos;
                    while self.lines.get(j).is_some_and(|l| classify(l.text) == LineKind::Comment) {
                        j += 1;
                    }
                    let follows = self.lines.get(j).map(|l| classify(l.text));
                    if matches!(follows, Some(LineKind::Blank | LineKind::Heading) | None) {
                        break;
                    }
                    while self.pos < j {
                        self.comment();
                    }
                }
                Some(LineKind::Blank | LineKind::Heading) | None => break,
                Some(_) => {
                    let line = self.bump();
                    self.prose_line(line);
                }
            }
        }
        self.b.finish_node();
    }

    fn prose_line(&mut self, line: Line) {
        self.b.start_node(PROSE_LINE.into());
        self.trimmed(TEXT, line.text, None);
        self.nl(line);
        self.b.finish_node();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(step: &Step) -> (Option<&'static str>, bool) {
        let code = match &step.action {
            Action::Misplaced { code, .. } | Action::Stray { code, .. } => Some(*code),
            _ => None,
        };
        (code, step.missing_kind_line)
    }

    #[test]
    fn description_after_a_gap_is_both_missing_kind_and_misplaced() {
        let s = step(Phase::AfterDescription, SecKind::Container, LineKind::Desc, None);
        assert_eq!(codes(&s), (Some("description-misplaced"), true));
        assert_eq!((s.phase, s.kind), (Phase::Structure, SecKind::Unknown));
    }

    #[test]
    fn tab_under_heading_is_both_missing_kind_and_tab() {
        let s = step(Phase::AfterHeading, SecKind::Container, LineKind::Tab, None);
        assert_eq!(codes(&s), (Some("tab-indent"), true));
    }

    #[test]
    fn kind_word_in_rationale_stays_prose() {
        let kind = SecKind::Known(SectionKind::Contest);
        let s = step(Phase::Prose, kind, LineKind::Prose, Some(SectionKind::Measure));
        assert!(matches!(s.action, Action::Paragraph));
        assert_eq!(s.kind, kind);
    }

    #[test]
    fn region_rejects_every_content_line() {
        let kind = SecKind::Known(SectionKind::Region);
        for line in [LineKind::Field, LineKind::Candidate, LineKind::Prose, LineKind::Desc] {
            let s = step(Phase::Structure, kind, line, None);
            assert_eq!(codes(&s), (Some("content-in-container"), false), "{line:?}");
        }
    }

    #[test]
    fn candidates_only_in_contests() {
        let s = step(Phase::Structure, SecKind::Known(SectionKind::Measure), LineKind::Candidate, None);
        assert_eq!(codes(&s).0, Some("candidate-in-non-contest"));
        let s = step(Phase::Structure, SecKind::Unknown, LineKind::Candidate, None);
        assert!(matches!(s.action, Action::Candidate));
    }
}
