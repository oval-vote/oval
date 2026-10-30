//! Inline markup (spec section 7): emphasis, strong, code, links, section
//! references, and backslash escapes. Input is the list of source segments
//! that make up a paragraph; they are joined with single spaces first, and
//! every node remembers its source range so diagnostics can point at it.

use crate::{Segment, TextRange, TextSize};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Inline {
    Text {
        text: String,
    },
    Emphasis {
        children: Vec<Inline>,
    },
    Strong {
        children: Vec<Inline>,
    },
    Code {
        text: String,
    },
    Link {
        children: Vec<Inline>,
        url: String,
        #[serde(skip)]
        range: TextRange,
    },
    Ref {
        children: Vec<Inline>,
        id: String,
        #[serde(skip)]
        range: TextRange,
    },
}

pub const ESCAPABLE: &str = "\\*_`[]()#>+-@/";

/// Where one segment landed in the joined string.
struct Span {
    joined: usize,
    source: u32,
    len: usize,
}

/// Maps offsets in the joined string back to source offsets.
struct Map {
    spans: Vec<Span>,
}

impl Map {
    fn source(&self, joined: usize) -> TextSize {
        let i = self.spans.partition_point(|s| s.joined <= joined).saturating_sub(1);
        let span = &self.spans[i];
        let d = (joined - span.joined).min(span.len);
        TextSize::from(span.source + d as u32)
    }
    fn range(&self, start: usize, end: usize) -> TextRange {
        TextRange::new(self.source(start), self.source(end))
    }
}

pub fn parse(segments: &[Segment]) -> Vec<Inline> {
    let mut joined = String::new();
    let mut spans = Vec::new();
    for (i, s) in segments.iter().enumerate() {
        if i > 0 {
            joined.push(' ');
        }
        spans.push(Span { joined: joined.len(), source: u32::from(s.range.start()), len: s.text.len() });
        joined.push_str(&s.text);
    }
    let map = Map { spans };
    P::new(&joined, &map).run(None).0
}

struct P<'a> {
    s: &'a [u8],
    src: &'a str,
    pos: usize,
    map: &'a Map,
    /// Openers (position, delimiter length) already known to have no closer,
    /// so an unclosed `*` deep in a paragraph is scanned once, not once per
    /// enclosing opener.
    unclosed: HashSet<(usize, usize)>,
    /// How many emphasis or strong spans enclose the current position. Spec
    /// section 7 allows nesting to one level, so this never exceeds
    /// `MAX_DEPTH`, which also bounds the recursion.
    depth: usize,
}

const MAX_DEPTH: usize = 2;

impl<'a> P<'a> {
    fn new(src: &'a str, map: &'a Map) -> Self {
        P { s: src.as_bytes(), src, pos: 0, map, unclosed: HashSet::new(), depth: 0 }
    }

    /// Parse until `close` is found at this nesting level (or end of input).
    /// Returns the children and whether the closer was found; `pos` is left
    /// after the closer when it was.
    fn run(&mut self, close: Option<&str>) -> (Vec<Inline>, bool) {
        let mut out = Vec::new();
        let mut text = String::new();
        let flush = |text: &mut String, out: &mut Vec<Inline>| {
            if !text.is_empty() {
                out.push(Inline::Text { text: std::mem::take(text) });
            }
        };
        while self.pos < self.s.len() {
            if let Some(c) = close {
                if self.src[self.pos..].starts_with(c) && self.closes_here(c) {
                    self.pos += c.len();
                    flush(&mut text, &mut out);
                    return (out, true);
                }
            }
            let b = self.s[self.pos];
            match b {
                b'\\'
                    if self.pos + 1 < self.s.len()
                        && ESCAPABLE.as_bytes().contains(&self.s[self.pos + 1]) =>
                {
                    text.push(self.s[self.pos + 1] as char);
                    self.pos += 2;
                }
                b'`' => {
                    if let Some(end) = self.src[self.pos + 1..].find('`') {
                        let code = &self.src[self.pos + 1..self.pos + 1 + end];
                        flush(&mut text, &mut out);
                        out.push(Inline::Code { text: code.to_string() });
                        self.pos += end + 2;
                    } else {
                        text.push('`');
                        self.pos += 1;
                    }
                }
                b'[' => {
                    let start = self.pos;
                    if let Some(node) = self.link(start) {
                        flush(&mut text, &mut out);
                        out.push(node);
                    } else {
                        self.pos = start + 1;
                        text.push('[');
                    }
                }
                b'*' => {
                    let start = self.pos;
                    let strong = self.src[self.pos..].starts_with("**");
                    let delim = if strong { "**" } else { "*" };
                    let key = (start, delim.len());
                    if self.depth < MAX_DEPTH && self.opens_here(delim) && !self.unclosed.contains(&key) {
                        self.pos += delim.len();
                        self.depth += 1;
                        let (children, closed) = self.run(Some(delim));
                        self.depth -= 1;
                        if closed {
                            flush(&mut text, &mut out);
                            out.push(if strong {
                                Inline::Strong { children }
                            } else {
                                Inline::Emphasis { children }
                            });
                            continue;
                        }
                        self.unclosed.insert(key);
                    }
                    // Not an opener, or no closer: literal.
                    self.pos = start + delim.len();
                    text.push_str(delim);
                }
                _ => {
                    let ch = self.src[self.pos..].chars().next().unwrap();
                    text.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        flush(&mut text, &mut out);
        (out, false)
    }

    fn link(&mut self, start: usize) -> Option<Inline> {
        // [text](target) with no nesting of brackets inside text.
        let rest = &self.src[start + 1..];
        let close = rest.find("](")?;
        if rest[..close].contains('[') {
            return None;
        }
        let after = &rest[close + 2..];
        let end = after.find(')')?;
        let target = &after[..end];
        if target.is_empty() || target.contains(char::is_whitespace) {
            return None;
        }
        // Parse the text with a sub-parser.
        let text_src = &self.src[start + 1..start + 1 + close];
        let sub_map = Map {
            spans: vec![Span {
                joined: 0,
                source: u32::from(self.map.source(start + 1)),
                len: text_src.len(),
            }],
        };
        let children = P::new(text_src, &sub_map).run(None).0;
        let total = 1 + close + 2 + end + 1;
        let range = self.map.range(start, start + total);
        self.pos = start + total;
        Some(if let Some(id) = target.strip_prefix('@') {
            Inline::Ref { children, id: id.to_string(), range }
        } else {
            Inline::Link { children, url: target.to_string(), range }
        })
    }

    /// Opening delimiter: not followed by whitespace, and not immediately
    /// followed by the same delimiter (which would be an empty span).
    fn opens_here(&self, delim: &str) -> bool {
        let after = self.s.get(self.pos + delim.len());
        matches!(after, Some(c) if !c.is_ascii_whitespace() && *c != b'*')
    }

    /// Closing delimiter: not preceded by whitespace.
    fn closes_here(&self, delim: &str) -> bool {
        let before = self.pos.checked_sub(1).map(|i| self.s[i]);
        let ok = matches!(before, Some(c) if !c.is_ascii_whitespace());
        // `*` must not be the start of `**` when closing emphasis.
        if ok && delim == "*" && self.s.get(self.pos + 1) == Some(&b'*') {
            return false;
        }
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(s: &str) -> Vec<Segment> {
        vec![Segment {
            text: s.to_string(),
            range: TextRange::new(TextSize::from(0), TextSize::from(s.len() as u32)),
        }]
    }
    fn text(s: &str) -> Inline {
        Inline::Text { text: s.to_string() }
    }

    #[test]
    fn plain() {
        assert_eq!(parse(&seg("hello world")), vec![text("hello world")]);
    }

    #[test]
    fn emphasis_and_strong() {
        assert_eq!(
            parse(&seg("a *b* **c** d")),
            vec![
                text("a "),
                Inline::Emphasis { children: vec![text("b")] },
                text(" "),
                Inline::Strong { children: vec![text("c")] },
                text(" d")
            ]
        );
    }

    #[test]
    fn unmatched_star_is_literal() {
        assert_eq!(parse(&seg("2 * 3 = 6")), vec![text("2 * 3 = 6")]);
        assert_eq!(parse(&seg("*open")), vec![text("*open")]);
    }

    #[test]
    fn underscore_is_literal() {
        assert_eq!(parse(&seg("_not_ emphasis")), vec![text("_not_ emphasis")]);
    }

    #[test]
    fn code_and_escapes() {
        assert_eq!(parse(&seg("`*x*` \\*y\\*")), vec![Inline::Code { text: "*x*".into() }, text(" *y*")]);
    }

    #[test]
    fn links_and_refs() {
        let out = parse(&seg("see [B](@b) or [site](https://x.org)"));
        assert!(matches!(&out[1], Inline::Ref { id, .. } if id == "b"));
        assert!(matches!(&out[3], Inline::Link { url, .. } if url == "https://x.org"));
        assert_eq!(parse(&seg("[not a link]")), vec![text("[not a link]")]);
    }

    #[test]
    fn nesting_stops_at_one_level() {
        // Inside strong, emphasis may open once more; the `*` before `c` is at
        // the depth limit and stays literal, so the next `*` closes the span.
        let out = parse(&seg("**a *b *c* d* e**"));
        let Inline::Strong { children } = &out[0] else { panic!("{out:?}") };
        assert_eq!(children, &[text("a "), Inline::Emphasis { children: vec![text("b *c")] }, text(" d* e")]);
    }

    #[test]
    fn unclosed_openers_are_linear() {
        // Every `*` opens and none closes; without memoizing failed openers this
        // rescans the tail once per opener.
        let start = std::time::Instant::now();
        for src in ["*a ".repeat(2000), "*".repeat(2000), "**a *b ".repeat(1000)] {
            let out = parse(&seg(&src));
            assert_eq!(out, vec![text(&src)]);
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(1), "{:?}", start.elapsed());
    }

    #[test]
    fn joins_segments_with_spaces() {
        let segs = vec![
            Segment { text: "a *b".into(), range: TextRange::new(TextSize::from(0), TextSize::from(4)) },
            Segment { text: "c* d".into(), range: TextRange::new(TextSize::from(10), TextSize::from(14)) },
        ];
        assert_eq!(
            parse(&segs),
            vec![text("a "), Inline::Emphasis { children: vec![text("b c")] }, text(" d")]
        );
    }
}
