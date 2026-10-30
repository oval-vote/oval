//! Line-oriented parser. Each source line is classified by its first
//! characters, then emitted as tokens under the node the current context
//! calls for. Nothing is ever dropped: a line that cannot appear where it is
//! becomes an `ERROR` node wrapping its normal parse, with a diagnostic.

use crate::diagnostic::Diagnostic;
use crate::kind::SyntaxKind::{self, *};
use crate::SyntaxNode;
use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

pub const KINDS: [&str; 4] = ["region", "contest", "measure", "retention"];

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
        diags: Vec::new(),
        open: Vec::new(),
    };
    p.file();
    Parse { green: p.b.finish(), diagnostics: p.diags }
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

/// The kind keyword when the line is a kind line.
fn kind_keyword(text: &str) -> Option<&'static str> {
    let word = text.split([',', ' ']).next()?;
    let k = KINDS.iter().find(|k| **k == word)?;
    let rest = text[k.len()..].trim_start();
    (rest.is_empty() || rest.starts_with(',')).then_some(k)
}

pub fn is_valid_id(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SecKind {
    Container,
    Region,
    Contest,
    Measure,
    Retention,
    /// Had content without a kind line; reported once, then parsed as a question.
    Unknown,
}

impl SecKind {
    fn is_question(self) -> bool {
        matches!(self, SecKind::Contest | SecKind::Measure | SecKind::Retention | SecKind::Unknown)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    AfterHeading,
    AfterDescription,
    Structure,
    Prose,
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
    diags: Vec<Diagnostic>,
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
        self.diags.push(Diagnostic::error(range, code, msg));
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
        while self.peek().is_some() {
            self.section_line();
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

    fn section_line(&mut self) {
        let line = self.peek().unwrap();
        let kind = classify(line.text);
        if kind == LineKind::Heading {
            self.heading();
            return;
        }
        let Some(cur) = self.open.last() else {
            // Cannot happen: HEADER consumed everything before the first heading.
            self.bump();
            self.error_line(line);
            return;
        };
        let (sec_kind, phase) = (cur.kind, cur.phase);
        match kind {
            LineKind::Blank => return self.blank(),
            LineKind::Comment => return self.comment(),
            _ => {}
        }
        if phase == Phase::AfterHeading && kind == LineKind::Desc {
            self.description();
            self.open.last_mut().unwrap().phase = Phase::AfterDescription;
            return;
        }
        if matches!(phase, Phase::AfterHeading | Phase::AfterDescription) {
            if kind == LineKind::Prose && kind_keyword(line.text).is_some() {
                self.kind_line();
                return;
            }
            // Content without a kind line.
            let cur = self.open.last_mut().unwrap();
            if cur.kind == SecKind::Container {
                cur.kind = SecKind::Unknown;
                self.error(line.range(), "content-in-container", "this section has no kind line, so it can only contain other sections; add `contest`, `measure`, `retention`, or `region` under the heading, or remove this");
            }
            self.open.last_mut().unwrap().phase = Phase::Structure;
            return self.section_line();
        }
        if sec_kind == SecKind::Region {
            self.bump();
            self.error(
                line.range(),
                "content-in-container",
                "a region holds sections, not fields, candidates, or rationale",
            );
            self.error_line(line);
            return;
        }
        match (phase, kind) {
            (Phase::Structure, LineKind::Field) => self.field(),
            (Phase::Structure, LineKind::Candidate) => {
                if sec_kind != SecKind::Contest && sec_kind != SecKind::Unknown {
                    self.error(
                        line.range(),
                        "candidate-in-non-contest",
                        "candidate lines are only allowed in a contest",
                    );
                    self.wrap_error(Parser::candidate);
                } else {
                    self.candidate();
                }
            }
            (Phase::Structure, LineKind::Prose) => {
                self.open.last_mut().unwrap().phase = Phase::Prose;
                self.paragraph();
            }
            (Phase::Prose, LineKind::Prose) => self.paragraph(),
            (Phase::Prose, LineKind::Field) => {
                self.error(
                    line.range(),
                    "structure-after-rationale",
                    "fields must come before the rationale; move this line above the first paragraph",
                );
                self.wrap_error(Parser::field);
            }
            (Phase::Prose, LineKind::Candidate) => {
                self.error(line.range(), "structure-after-rationale", "candidate lines must come before the rationale; move this line above the first paragraph");
                self.wrap_error(Parser::candidate);
            }
            (_, LineKind::Desc) => {
                self.error(
                    line.range(),
                    "description-misplaced",
                    "a `>` description goes directly under the heading",
                );
                self.wrap_error(Parser::description);
            }
            (_, LineKind::Continuation) => {
                self.bump();
                self.error(line.range(), "stray-continuation", "an indented line continues the field or candidate line directly above it; nothing is above this one");
                self.error_line(line);
            }
            (_, LineKind::Tab) => {
                self.bump();
                self.error(line.range(), "tab-indent", "lines are indented with spaces, not tabs");
                self.error_line(line);
            }
            (_, LineKind::Heading | LineKind::Blank | LineKind::Comment) => unreachable!(),
            (Phase::AfterHeading | Phase::AfterDescription, _) => {
                unreachable!("phase advanced above")
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

    fn kind_line(&mut self) {
        let line = self.bump();
        let kw = kind_keyword(line.text).unwrap();
        let cur = self.open.last_mut().unwrap();
        cur.kind = match kw {
            "region" => SecKind::Region,
            "contest" => SecKind::Contest,
            "measure" => SecKind::Measure,
            _ => SecKind::Retention,
        };
        cur.phase = Phase::Structure;
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
