//! Lower a syntax tree into the data model, applying spec section 6.

use crate::validate::{is_full_date, is_http_url, is_language_tag};
use crate::*;
use oval_syntax::ast::{self, Marker};
use oval_syntax::inline::{self, Inline};
use oval_syntax::{parse, Diagnostic, Parse, Segment, TextRange, TextSize};
use std::collections::HashMap;

/// A parsed and lowered file. `guide` is always produced, best effort, so
/// that editors have an outline even while the file has errors.
#[derive(Clone, Debug)]
pub struct Loaded {
    pub parse: Parse,
    pub guide: Guide,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn load(text: &str) -> Loaded {
    let parse = parse(text);
    let root = ast::Root::cast(parse.syntax()).expect("root");
    let mut l = Lower {
        diags: parse.diagnostics.clone(),
        ids: HashMap::new(),
        refs: Vec::new(),
    };
    let guide = l.guide(&root);
    for (id, range) in std::mem::take(&mut l.refs) {
        if !l.ids.contains_key(&id) {
            l.error(
                range,
                "unresolved-ref",
                format!("no section has the id `{id}`"),
            );
        }
    }
    let mut diagnostics = l.diags;
    diagnostics.sort_by_key(|d| (d.range.start(), d.range.end()));
    Loaded {
        parse,
        guide,
        diagnostics,
    }
}

struct Lower {
    diags: Vec<Diagnostic>,
    ids: HashMap<String, TextRange>,
    refs: Vec<(String, TextRange)>,
}

pub const CONTEST_FIELDS: &[&str] = &["note", "vote", "party"];
pub const MEASURE_FIELDS: &[&str] = &[
    "note",
    "vote",
    "placed by",
    "yes",
    "no",
    "threshold",
    "supporters",
    "opponents",
];
pub const RETENTION_FIELDS: &[&str] = &["note", "vote", "designation"];
pub const HEADER_FIELDS: &[&str] = &["name", "date", "author", "updated", "language"];

impl Lower {
    fn error(&mut self, range: TextRange, code: &'static str, msg: impl Into<String>) {
        self.diags.push(Diagnostic::error(range, code, msg));
    }
    fn warning(&mut self, range: TextRange, code: &'static str, msg: impl Into<String>) {
        self.diags.push(Diagnostic::warning(range, code, msg));
    }

    fn inline(&mut self, segments: &[Segment]) -> Vec<Inline> {
        let nodes = inline::parse(segments);
        self.collect_refs(&nodes);
        nodes
    }

    fn collect_refs(&mut self, nodes: &[Inline]) {
        for n in nodes {
            match n {
                Inline::Ref {
                    children,
                    id,
                    range,
                } => {
                    self.refs.push((id.clone(), *range));
                    self.collect_refs(children);
                }
                Inline::Link {
                    children,
                    url,
                    range,
                } => {
                    if !is_http_url(url) {
                        self.error(
                            *range,
                            "bad-url",
                            "links must be absolute http or https URLs",
                        );
                    }
                    self.collect_refs(children);
                }
                Inline::Emphasis { children } | Inline::Strong { children } => {
                    self.collect_refs(children)
                }
                Inline::Text { .. } | Inline::Code { .. } => {}
            }
        }
    }

    /// Gather fields into a map, reporting duplicates and unknown keys.
    /// `x-` fields go to `ext` instead.
    fn fields(
        &mut self,
        fields: impl Iterator<Item = ast::Field>,
        allowed: &[&str],
        ext: &mut Extensions,
    ) -> HashMap<String, ast::Field> {
        let mut map = HashMap::new();
        for f in fields {
            let Some(key) = f.key() else { continue };
            if key.text.starts_with("x-") {
                match ext.fields.entry(key.text.clone()) {
                    std::collections::btree_map::Entry::Occupied(_) => {
                        self.error(
                            key.range,
                            "duplicate-field",
                            format!("`{}` appears more than once in this section", key.text),
                        );
                    }
                    std::collections::btree_map::Entry::Vacant(e) => {
                        e.insert(f.value());
                    }
                }
                continue;
            }
            if !allowed.contains(&key.text.as_str()) {
                self.error(
                    key.range,
                    "unknown-field",
                    format!(
                        "`{}` is not a field here; allowed: {}",
                        key.text,
                        allowed.join(", ")
                    ),
                );
                continue;
            }
            if map.contains_key(&key.text) {
                self.error(
                    key.range,
                    "duplicate-field",
                    format!("`{}` appears more than once in this section", key.text),
                );
                continue;
            }
            map.insert(key.text, f);
        }
        map
    }

    /// Check tags against `allowed` (exact words); returns the ones present.
    /// `x-` tags go to `ext`.
    fn tags(&mut self, tags: &[Segment], allowed: &[&str], ext: &mut Extensions) -> Vec<String> {
        let mut seen = Vec::new();
        for t in tags {
            if t.text.starts_with("x-") {
                if ext.tags.contains(&t.text) {
                    self.error(
                        t.range,
                        "duplicate-tag",
                        format!("`{}` is repeated", t.text),
                    );
                } else {
                    ext.tags.push(t.text.clone());
                }
                continue;
            }
            if !allowed.contains(&t.text.as_str()) {
                self.error(
                    t.range,
                    "unknown-tag",
                    format!("`{}` is not a tag here", t.text),
                );
                continue;
            }
            if seen.contains(&t.text) {
                self.error(
                    t.range,
                    "duplicate-tag",
                    format!("`{}` is repeated", t.text),
                );
                continue;
            }
            seen.push(t.text.clone());
        }
        seen
    }

    fn guide(&mut self, root: &ast::Root) -> Guide {
        let version = root.magic().and_then(|m| m.version()).unwrap_or(1);
        let header_range = root
            .header()
            .map_or(TextRange::empty(TextSize::from(0)), |h| h.range());
        let first_line = root.magic().map_or(header_range, |m| m.range());
        let mut extensions = Extensions::default();
        let fields = match root.header() {
            Some(h) => self.fields(h.fields(), HEADER_FIELDS, &mut extensions),
            None => HashMap::new(),
        };
        let take = |k: &str| fields.get(k).map(|f| (f.value(), f.range()));

        let name = match take("name") {
            Some((v, _)) => v,
            None => {
                self.error(
                    first_line,
                    "missing-field",
                    "the header needs a `name:` line",
                );
                String::new()
            }
        };
        let date = match take("date") {
            Some((v, r)) => {
                if !is_full_date(&v) {
                    self.error(r, "bad-date", "dates are written YYYY-MM-DD");
                }
                v
            }
            None => {
                self.error(
                    first_line,
                    "missing-field",
                    "the header needs a `date:` line",
                );
                String::new()
            }
        };
        let author = match take("author") {
            Some((v, r)) => {
                let (n, url) = match v.strip_suffix('>').and_then(|s| s.rsplit_once(" <")) {
                    Some((n, u)) => (n.trim().to_string(), Some(u.to_string())),
                    None => (v.clone(), None),
                };
                if n.is_empty() {
                    self.error(r, "bad-author", "author is `Name` or `Name <url>`");
                }
                if let Some(u) = &url {
                    if !is_http_url(u) {
                        self.error(
                            r,
                            "bad-url",
                            "the author url must be an absolute http or https URL",
                        );
                    }
                }
                Author { name: n, url }
            }
            None => {
                self.error(
                    first_line,
                    "missing-field",
                    "the header needs an `author:` line",
                );
                Author {
                    name: String::new(),
                    url: None,
                }
            }
        };
        let updated = take("updated").map(|(v, r)| {
            if !is_full_date(&v) {
                self.error(r, "bad-date", "dates are written YYYY-MM-DD");
            }
            v
        });
        let language = match take("language") {
            Some((v, r)) => {
                if !is_language_tag(&v) {
                    self.error(
                        r,
                        "bad-language",
                        "language is a BCP 47 tag such as `en` or `es-MX`",
                    );
                }
                v
            }
            None => "en".to_string(),
        };
        let children = root.sections().filter_map(|s| self.node(&s)).collect();
        Guide {
            version,
            name,
            date,
            author,
            updated,
            language,
            extensions,
            children,
        }
    }

    fn section_id(&mut self, heading: &ast::Heading) -> (String, String) {
        let name = heading.name().map(|n| n.text).unwrap_or_default();
        let (id, range) = match heading.id() {
            Some(t) => (t.text().to_string(), t.text_range()),
            None => {
                let s = slug(&name);
                let r = heading.name().map_or(heading.range(), |n| n.range);
                self.warning(r, "derived-id", format!("no `@id`; derived `@{s}` from the name, which will change if the name does"));
                (s, r)
            }
        };
        if let Some(first) = self.ids.get(&id) {
            let first = *first;
            self.error(
                range,
                "duplicate-id",
                format!(
                    "`@{id}` is also used at offset {}",
                    u32::from(first.start())
                ),
            );
        } else {
            self.ids.insert(id.clone(), range);
        }
        (id, name)
    }

    fn node(&mut self, s: &ast::Section) -> Option<Node> {
        let heading = s.heading()?;
        let span = Span {
            full: s.range(),
            line: heading.range(),
        };
        let (id, name) = self.section_id(&heading);
        let description = s.description().map(|d| self.inline(&d.segments()));
        let kind = s.kind();
        let tags = s.kind_line().map(|k| k.tags()).unwrap_or_default();
        Some(match kind.as_deref() {
            None | Some("region") => {
                let region = kind.is_some();
                let mut extensions = Extensions::default();
                if region {
                    self.tags(&tags, &[], &mut extensions);
                }
                let children = s.sections().filter_map(|c| self.node(&c)).collect();
                Node::Section(Section {
                    span,
                    id,
                    name,
                    description,
                    region,
                    extensions,
                    children,
                })
            }
            Some("contest") => Node::Contest(self.contest(s, span, id, name, description, &tags)),
            Some("measure") => Node::Measure(self.measure(s, span, id, name, description, &tags)),
            Some("retention") => {
                Node::Retention(self.retention(s, span, id, name, description, &tags))
            }
            Some(_) => return None,
        })
    }

    fn rationale(&mut self, s: &ast::Section) -> Vec<Vec<Inline>> {
        s.paragraphs().map(|p| self.inline(&p.segments())).collect()
    }

    fn vote(&mut self, fields: &HashMap<String, ast::Field>, contest: bool) -> Option<Vote> {
        let f = fields.get("vote")?;
        let v = f.value();
        let vote = match v.as_str() {
            "undecided" => Vote::Undecided,
            "none" => Vote::None,
            "yes" if !contest => Vote::Yes,
            "no" if !contest => Vote::No,
            _ => {
                let allowed = if contest {
                    "undecided or none"
                } else {
                    "yes, no, undecided, or none"
                };
                self.error(
                    f.range(),
                    "bad-vote-value",
                    format!("`vote:` is {allowed}; got `{v}`"),
                );
                Vote::Undecided
            }
        };
        Some(vote)
    }

    fn list(&mut self, fields: &HashMap<String, ast::Field>, key: &str) -> Vec<String> {
        let Some(f) = fields.get(key) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for item in f.value().split(',') {
            let item = item.trim();
            if item.is_empty() {
                self.error(
                    f.range(),
                    "empty-list-item",
                    format!("`{key}:` has an empty item between commas"),
                );
            } else {
                out.push(item.to_string());
            }
        }
        out
    }

    fn contest(
        &mut self,
        s: &ast::Section,
        span: Span,
        id: String,
        name: String,
        description: Option<Vec<Inline>>,
        tags: &[Segment],
    ) -> Contest {
        // Tags: fixed words plus the two parameterized forms.
        let mut extensions = Extensions::default();
        let mut partisan = false;
        let mut write_ins = false;
        let mut unverified = false;
        let mut ranked: Option<TextRange> = None;
        let mut seats: Option<(u32, TextRange)> = None;
        let mut max: Option<(u32, TextRange)> = None;
        let mut seen: Vec<&str> = Vec::new();
        for t in tags {
            let text = t.text.as_str();
            let word = if let Some(n) = text.strip_suffix(" seats") {
                match n.parse::<u32>() {
                    Ok(n) if n >= 2 => seats = Some((n, t.range)),
                    _ => self.error(
                        t.range,
                        "bad-number",
                        "`N seats` needs a whole number of at least 2",
                    ),
                }
                "seats"
            } else if let Some(n) = text.strip_prefix("rank up to ") {
                match n.parse::<u32>() {
                    Ok(n) if n >= 1 => max = Some((n, t.range)),
                    _ => self.error(
                        t.range,
                        "bad-number",
                        "`rank up to N` needs a whole number of at least 1",
                    ),
                }
                "rank up to"
            } else {
                match text {
                    "partisan" => partisan = true,
                    "ranked" => ranked = Some(t.range),
                    "write-ins" => write_ins = true,
                    "unverified" => unverified = true,
                    _ if text.starts_with("x-") => {
                        if extensions.tags.contains(&t.text) {
                            self.error(t.range, "duplicate-tag", format!("`{text}` is repeated"));
                        } else {
                            extensions.tags.push(t.text.clone());
                        }
                        continue;
                    }
                    _ => {
                        self.error(
                            t.range,
                            "unknown-tag",
                            format!("`{text}` is not a contest tag; expected partisan, ranked, N seats, rank up to N, write-ins, or unverified"),
                        );
                        continue;
                    }
                }
                text
            };
            if seen.contains(&word) {
                self.error(t.range, "duplicate-tag", format!("`{text}` is repeated"));
            }
            seen.push(word);
        }
        if let (Some(_), Some((_, r))) = (ranked, seats) {
            self.error(
                r,
                "conflicting-tags",
                "`ranked` and `N seats` cannot both be set",
            );
        }
        if let (None, Some((_, r))) = (ranked, max) {
            self.error(r, "tag-requires", "`rank up to N` needs `ranked`");
        }
        let method = if ranked.is_some() {
            Method::Ranked {
                max: max.map(|m| m.0),
            }
        } else if let Some((n, _)) = seats {
            Method::Multiple { seats: n }
        } else {
            Method::Single
        };

        let fields = self.fields(s.fields(), CONTEST_FIELDS, &mut extensions);
        let party = fields.get("party").map(|f| f.value());
        let note = fields.get("note").map(|f| self.inline(&f.segments()));
        let vote = self.vote(&fields, true);

        // Candidates.
        let mut candidates = Vec::new();
        let mut cand_ids: HashMap<String, TextRange> = HashMap::new();
        let mut picks = 0u32;
        let mut ranks: Vec<(u32, TextRange)> = Vec::new();
        for c in s.candidates() {
            let Some(cname) = c.name() else { continue };
            let marker = c.marker().unwrap_or(Marker::Neutral);
            let (cid, cid_range) = match c.id() {
                Some(t) => (t.text().to_string(), t.text_range()),
                None => (slug(&cname.text), cname.range),
            };
            if cand_ids.contains_key(&cid) {
                self.error(
                    cid_range,
                    "duplicate-candidate-id",
                    format!("two candidates in this contest have the id `{cid}`; give one an explicit `@id`"),
                );
            } else {
                cand_ids.insert(cid.clone(), cid_range);
            }
            let mut party = None;
            let mut incumbent = false;
            let mut write_in = false;
            for l in c.labels() {
                match l.text.as_str() {
                    "incumbent" => incumbent = true,
                    "write-in" => {
                        write_in = true;
                        if !write_ins {
                            self.error(
                                l.range,
                                "write-in-not-allowed",
                                "a write-in candidate needs the `write-ins` tag on the contest",
                            );
                        }
                    }
                    other => {
                        if party.is_some() {
                            self.error(l.range, "multiple-party-labels", "only one label may be a party; the others are `incumbent` and `write-in`");
                        }
                        party = Some(other.to_string());
                    }
                }
            }
            let position = match marker {
                Marker::Pick => {
                    picks += 1;
                    if ranked.is_some() {
                        self.error(
                            c.marker_range(),
                            "plus-in-ranked",
                            "in a ranked contest picks are numbered: `1.`, `2.`, ...",
                        );
                    } else if picks > seats.map_or(1, |s| s.0) {
                        let n = seats.map_or(1, |s| s.0);
                        self.error(
                            c.marker_range(),
                            "too-many-picks",
                            format!(
                                "this contest fills {n} seat{}; too many `+` lines",
                                if n == 1 { "" } else { "s" }
                            ),
                        );
                    }
                    Position::Pick
                }
                Marker::Rank(n) => {
                    if ranked.is_none() {
                        self.error(
                            c.marker_range(),
                            "rank-in-unranked",
                            "rank numbers are only for `ranked` contests; use `+`",
                        );
                    }
                    if n == 0 {
                        self.error(c.marker_range(), "bad-rank", "ranks start at 1");
                    }
                    ranks.push((n, c.marker_range()));
                    Position::Rank { rank: n }
                }
                Marker::Neutral => Position::Neutral,
                Marker::Avoid => Position::Avoid,
            };
            let note = c.note();
            candidates.push(Candidate {
                span: Span {
                    full: c.range(),
                    line: c.range(),
                },
                id: cid,
                name: cname.text,
                party,
                incumbent,
                write_in,
                designation: c.designation().map(|d| d.text),
                note: (!note.is_empty()).then(|| self.inline(&note)),
                position,
            });
        }
        // Rank numbering.
        let mut distinct: Vec<u32> = ranks.iter().map(|r| r.0).filter(|n| *n > 0).collect();
        distinct.sort_unstable();
        distinct.dedup();
        for (i, n) in distinct.iter().enumerate() {
            if *n != i as u32 + 1 {
                let r = ranks.iter().find(|r| r.0 == *n).map(|r| r.1).unwrap();
                self.error(r, "rank-gap", format!("rank {n} follows rank {i}; ranks run 1, 2, 3 with no gaps (ties share a number)"));
                break;
            }
        }
        if let (Some((m, _)), Some(top)) = (max, distinct.last()) {
            if *top > m {
                let r = ranks.iter().find(|r| r.0 == *top).map(|r| r.1).unwrap();
                self.error(
                    r,
                    "rank-exceeds-max",
                    format!("the ballot allows at most {m} rankings"),
                );
            }
        }
        // Decision.
        let picked = picks > 0 || !ranks.is_empty();
        let decision = match (picked, vote) {
            (true, Some(_)) => {
                self.error(
                    fields["vote"].range(),
                    "vote-with-picks",
                    "this contest has picks, so `vote:` must be removed",
                );
                Decision::Decided
            }
            (true, None) => Decision::Decided,
            (false, Some(Vote::None)) => Decision::None,
            (false, Some(_)) => Decision::Undecided,
            (false, None) => {
                let r = s.heading().map_or(s.range(), |h| h.range());
                self.error(
                    r,
                    "missing-vote",
                    "no candidate is picked; say `vote: undecided` or `vote: none`",
                );
                Decision::Undecided
            }
        };
        let rationale = self.rationale(s);
        candidates.sort_by_key(candidate_order);
        Contest {
            span,
            id,
            name,
            description,
            partisan,
            method,
            write_ins,
            unverified,
            party,
            note,
            candidates,
            decision,
            rationale,
            extensions,
        }
    }

    fn measure(
        &mut self,
        s: &ast::Section,
        span: Span,
        id: String,
        name: String,
        description: Option<Vec<Inline>>,
        tags: &[Segment],
    ) -> Measure {
        let mut extensions = Extensions::default();
        let unverified = self
            .tags(tags, &["unverified"], &mut extensions)
            .iter()
            .any(|t| t == "unverified");
        let fields = self.fields(s.fields(), MEASURE_FIELDS, &mut extensions);
        let plain = |k: &str| fields.get(k).map(|f| f.value());
        let vote = self.required_vote(s, &fields);
        Measure {
            span,
            id,
            name,
            description,
            unverified,
            placed_by: plain("placed by"),
            yes: plain("yes"),
            no: plain("no"),
            threshold: plain("threshold"),
            supporters: self.list(&fields, "supporters"),
            opponents: self.list(&fields, "opponents"),
            note: fields.get("note").map(|f| self.inline(&f.segments())),
            vote,
            rationale: self.rationale(s),
            extensions,
        }
    }

    fn retention(
        &mut self,
        s: &ast::Section,
        span: Span,
        id: String,
        name: String,
        description: Option<Vec<Inline>>,
        tags: &[Segment],
    ) -> Retention {
        let mut extensions = Extensions::default();
        let unverified = self
            .tags(tags, &["unverified"], &mut extensions)
            .iter()
            .any(|t| t == "unverified");
        let fields = self.fields(s.fields(), RETENTION_FIELDS, &mut extensions);
        let vote = self.required_vote(s, &fields);
        Retention {
            span,
            id,
            name,
            description,
            unverified,
            designation: fields.get("designation").map(|f| f.value()),
            note: fields.get("note").map(|f| self.inline(&f.segments())),
            vote,
            rationale: self.rationale(s),
            extensions,
        }
    }

    fn required_vote(&mut self, s: &ast::Section, fields: &HashMap<String, ast::Field>) -> Vote {
        match self.vote(fields, false) {
            Some(v) => v,
            None => {
                let r = s.heading().map_or(s.range(), |h| h.range());
                self.error(
                    r,
                    "missing-vote",
                    "every measure and retention needs `vote: yes`, `no`, `undecided`, or `none`",
                );
                Vote::Undecided
            }
        }
    }
}
