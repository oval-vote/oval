//! Lower a syntax tree into the data model, applying spec section 6.

use crate::validate::{is_full_date, is_http_url, is_language_tag};
use crate::{
    candidate_order, slug, Author, Candidate, Contest, Decision, Extensions, Guide, Measure, Method, Node,
    Position, Retention, Section, Span, Vote,
};
use oval_syntax::ast::{self, Marker, SectionKind};
use oval_syntax::inline::{self, Inline};
use oval_syntax::{parse, Diagnostic, Diagnostics, Segment, SyntaxNode, TextRange, TextSize};
use rowan::GreenNode;
use std::collections::{BTreeMap, HashMap};

/// A parsed and lowered file. `guide` is always produced, best effort, so
/// that editors have an outline even while the file has errors.
/// `diagnostics` holds the parser's and the model's findings together, in
/// position order.
#[derive(Clone, Debug)]
pub struct Loaded {
    green: GreenNode,
    pub guide: Guide,
    pub diagnostics: Vec<Diagnostic>,
}

impl Loaded {
    /// The lossless syntax tree the model was built from.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
}

pub fn load(text: &str) -> Loaded {
    let parse = parse(text);
    let root = ast::Root::cast(parse.syntax()).expect("the parser always emits a root");
    let mut l = Lower { diags: Diagnostics::from(parse.diagnostics), ids: HashMap::new(), refs: Vec::new() };
    let guide = l.guide(&root);
    l.resolve_refs();
    l.diags.sort();
    Loaded { green: parse.green, guide, diagnostics: l.diags.into_vec() }
}

struct Lower {
    diags: Diagnostics,
    ids: HashMap<String, TextRange>,
    refs: Vec<(String, TextRange)>,
}

pub const CONTEST_FIELDS: &[&str] = &["note", "vote", "party"];
pub const MEASURE_FIELDS: &[&str] =
    &["note", "vote", "placed by", "yes", "no", "threshold", "supporters", "opponents"];
pub const RETENTION_FIELDS: &[&str] = &["note", "vote", "designation"];
pub const HEADER_FIELDS: &[&str] = &["name", "date", "author", "updated", "language"];

type Fields = HashMap<String, ast::Field>;

/// What a contest's kind line said.
struct ContestTags {
    partisan: bool,
    write_ins: bool,
    unverified: bool,
    ranked: Option<TextRange>,
    seats: Option<(u32, TextRange)>,
    max: Option<(u32, TextRange)>,
    extensions: Extensions,
}

impl ContestTags {
    fn method(&self) -> Method {
        if self.ranked.is_some() {
            Method::Ranked { max: self.max.map(|m| m.0) }
        } else if let Some((n, _)) = self.seats {
            Method::Multiple { seats: n }
        } else {
            Method::Single
        }
    }
    fn seat_count(&self) -> u32 {
        self.seats.map_or(1, |s| s.0)
    }
}

/// Running counts across one contest's candidate lines.
#[derive(Default)]
struct Tally {
    ids: HashMap<String, TextRange>,
    picks: u32,
    /// Every rank number used, with the range of its marker.
    ranks: Vec<(u32, TextRange)>,
}

impl Lower {
    fn error(&mut self, range: TextRange, code: &'static str, msg: impl Into<String>) {
        self.diags.error(range, code, msg);
    }
    fn warning(&mut self, range: TextRange, code: &'static str, msg: impl Into<String>) {
        self.diags.warning(range, code, msg);
    }

    fn resolve_refs(&mut self) {
        for (id, range) in std::mem::take(&mut self.refs) {
            if !self.ids.contains_key(&id) {
                self.error(range, "unresolved-ref", format!("no section has the id `{id}`"));
            }
        }
    }

    fn inline(&mut self, segments: &[Segment]) -> Vec<Inline> {
        let nodes = inline::parse(segments);
        self.collect_refs(&nodes);
        nodes
    }

    fn collect_refs(&mut self, nodes: &[Inline]) {
        for n in nodes {
            match n {
                Inline::Ref { children, id, range } => {
                    self.refs.push((id.clone(), *range));
                    self.collect_refs(children);
                }
                Inline::Link { children, url, range } => {
                    if !is_http_url(url) {
                        self.error(*range, "bad-url", "links must be absolute http or https URLs");
                    }
                    self.collect_refs(children);
                }
                Inline::Emphasis { children } | Inline::Strong { children } => self.collect_refs(children),
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
    ) -> Fields {
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
                    format!("`{}` is not a field here; allowed: {}", key.text, allowed.join(", ")),
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
                self.extension_tag(t, ext);
                continue;
            }
            if !allowed.contains(&t.text.as_str()) {
                self.error(t.range, "unknown-tag", format!("`{}` is not a tag here", t.text));
                continue;
            }
            if seen.contains(&t.text) {
                self.error(t.range, "duplicate-tag", format!("`{}` is repeated", t.text));
                continue;
            }
            seen.push(t.text.clone());
        }
        seen
    }

    fn extension_tag(&mut self, t: &Segment, ext: &mut Extensions) {
        if ext.tags.contains(&t.text) {
            self.error(t.range, "duplicate-tag", format!("`{}` is repeated", t.text));
        } else {
            ext.tags.push(t.text.clone());
        }
    }

    fn guide(&mut self, root: &ast::Root) -> Guide {
        let version = root.magic().and_then(|m| m.version()).unwrap_or(1);
        let header_range = root.header().map_or(TextRange::empty(TextSize::from(0)), |h| h.range());
        let first_line = root.magic().map_or(header_range, |m| m.range());
        let mut extensions = Extensions::default();
        let fields = match root.header() {
            Some(h) => self.fields(h.fields(), HEADER_FIELDS, &mut extensions),
            None => HashMap::new(),
        };
        let take = |k: &str| fields.get(k).map(|f| (f.value(), f.range()));

        let name = if let Some((v, _)) = take("name") {
            v
        } else {
            self.error(first_line, "missing-field", "the header needs a `name:` line");
            String::new()
        };
        let date = if let Some((v, r)) = take("date") {
            if !is_full_date(&v) {
                self.error(r, "bad-date", "dates are written YYYY-MM-DD");
            }
            v
        } else {
            self.error(first_line, "missing-field", "the header needs a `date:` line");
            String::new()
        };
        let author = if let Some((v, r)) = take("author") {
            let (n, url) = match v.strip_suffix('>').and_then(|s| s.rsplit_once(" <")) {
                Some((n, u)) => (n.trim().to_string(), Some(u.to_string())),
                None => (v.clone(), None),
            };
            if n.is_empty() {
                self.error(r, "bad-author", "author is `Name` or `Name <url>`");
            }
            if let Some(u) = &url {
                if !is_http_url(u) {
                    self.error(r, "bad-url", "the author url must be an absolute http or https URL");
                }
            }
            Author { name: n, url }
        } else {
            self.error(first_line, "missing-field", "the header needs an `author:` line");
            Author { name: String::new(), url: None }
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
                    self.error(r, "bad-language", "language is a BCP 47 tag such as `en` or `es-MX`");
                }
                v
            }
            None => "en".to_string(),
        };
        let children = root.sections().filter_map(|s| self.node(&s)).collect();
        Guide { version, name, date, author, updated, language, extensions, children }
    }

    fn section_id(&mut self, heading: &ast::Heading) -> (String, String) {
        let name = heading.name().map(|n| n.text).unwrap_or_default();
        let (id, range) = if let Some(t) = heading.id() {
            (t.text().to_string(), t.text_range())
        } else {
            let s = slug(&name);
            let r = heading.name().map_or(heading.range(), |n| n.range);
            self.warning(
                r,
                "derived-id",
                format!("no `@id`; derived `@{s}` from the name, which will change if the name does"),
            );
            (s, r)
        };
        if let Some(first) = self.ids.get(&id) {
            let first = *first;
            self.error(
                range,
                "duplicate-id",
                format!("`@{id}` is also used at offset {}", u32::from(first.start())),
            );
        } else {
            self.ids.insert(id.clone(), range);
        }
        (id, name)
    }

    fn node(&mut self, s: &ast::Section) -> Option<Node> {
        let heading = s.heading()?;
        let span = Span { full: s.range(), line: heading.range() };
        let (id, name) = self.section_id(&heading);
        let description = s.description().map(|d| self.inline(&d.segments()));
        let kind = s.kind();
        let tags = s.kind_line().map(|k| k.tags()).unwrap_or_default();
        Some(match kind {
            None | Some(SectionKind::Region) => {
                let region = kind.is_some();
                let mut extensions = Extensions::default();
                if region {
                    self.tags(&tags, &[], &mut extensions);
                }
                let children = s.sections().filter_map(|c| self.node(&c)).collect();
                Node::Section(Section { span, id, name, description, region, extensions, children })
            }
            Some(SectionKind::Contest) => Node::Contest(self.contest(s, span, id, name, description, &tags)),
            Some(SectionKind::Measure) => Node::Measure(self.measure(s, span, id, name, description, &tags)),
            Some(SectionKind::Retention) => {
                Node::Retention(self.retention(s, span, id, name, description, &tags))
            }
        })
    }

    fn rationale(&mut self, s: &ast::Section) -> Vec<Vec<Inline>> {
        s.paragraphs().map(|p| self.inline(&p.segments())).collect()
    }

    /// The `vote:` field, if present, with its range.
    fn vote(&mut self, fields: &Fields, contest: bool) -> Option<(Vote, TextRange)> {
        let f = fields.get("vote")?;
        let v = f.value();
        let vote = match v.as_str() {
            "undecided" => Vote::Undecided,
            "none" => Vote::None,
            "yes" if !contest => Vote::Yes,
            "no" if !contest => Vote::No,
            _ => {
                let allowed = if contest { "undecided or none" } else { "yes, no, undecided, or none" };
                self.error(f.range(), "bad-vote-value", format!("`vote:` is {allowed}; got `{v}`"));
                Vote::Undecided
            }
        };
        Some((vote, f.range()))
    }

    fn required_vote(&mut self, s: &ast::Section, fields: &Fields) -> Vote {
        if let Some((v, _)) = self.vote(fields, false) {
            v
        } else {
            let r = s.heading().map_or(s.range(), |h| h.range());
            self.error(
                r,
                "missing-vote",
                "every measure and retention needs `vote: yes`, `no`, `undecided`, or `none`",
            );
            Vote::Undecided
        }
    }

    fn list(&mut self, fields: &Fields, key: &str) -> Vec<String> {
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
        let mut tags = self.contest_tags(tags);
        let fields = self.fields(s.fields(), CONTEST_FIELDS, &mut tags.extensions);
        let party = fields.get("party").map(ast::Field::value);
        let note = fields.get("note").map(|f| self.inline(&f.segments()));
        let vote = self.vote(&fields, true);

        let mut tally = Tally::default();
        let mut candidates: Vec<Candidate> =
            s.candidates().filter_map(|c| self.candidate(&c, &tags, &mut tally)).collect();
        candidates.sort_by_key(candidate_order);
        self.check_ranks(&tally, tags.max);
        let picked = tally.picks > 0 || !tally.ranks.is_empty();
        let decision = self.decision(s, picked, vote);
        let rationale = self.rationale(s);
        Contest {
            span,
            id,
            name,
            description,
            partisan: tags.partisan,
            method: tags.method(),
            write_ins: tags.write_ins,
            unverified: tags.unverified,
            party,
            note,
            candidates,
            decision,
            rationale,
            extensions: tags.extensions,
        }
    }

    /// Contest tags: fixed words plus the two parameterized forms.
    fn contest_tags(&mut self, tags: &[Segment]) -> ContestTags {
        let mut out = ContestTags {
            partisan: false,
            write_ins: false,
            unverified: false,
            ranked: None,
            seats: None,
            max: None,
            extensions: Extensions::default(),
        };
        let mut seen: Vec<&str> = Vec::new();
        for t in tags {
            let text = t.text.as_str();
            let word = if let Some(n) = text.strip_suffix(" seats") {
                match n.parse::<u32>() {
                    Ok(n) if n >= 2 => out.seats = Some((n, t.range)),
                    _ => self.error(t.range, "bad-number", "`N seats` needs a whole number of at least 2"),
                }
                "seats"
            } else if let Some(n) = text.strip_prefix("rank up to ") {
                match n.parse::<u32>() {
                    Ok(n) if n >= 1 => out.max = Some((n, t.range)),
                    _ => {
                        self.error(
                            t.range,
                            "bad-number",
                            "`rank up to N` needs a whole number of at least 1",
                        );
                    }
                }
                "rank up to"
            } else {
                match text {
                    "partisan" => out.partisan = true,
                    "ranked" => out.ranked = Some(t.range),
                    "write-ins" => out.write_ins = true,
                    "unverified" => out.unverified = true,
                    _ if text.starts_with("x-") => {
                        self.extension_tag(t, &mut out.extensions);
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
        if let (Some(_), Some((_, r))) = (out.ranked, out.seats) {
            self.error(r, "conflicting-tags", "`ranked` and `N seats` cannot both be set");
        }
        if let (None, Some((_, r))) = (out.ranked, out.max) {
            self.error(r, "tag-requires", "`rank up to N` needs `ranked`");
        }
        out
    }

    /// One candidate line. Updates the tally used by the contest-level checks.
    fn candidate(&mut self, c: &ast::Candidate, tags: &ContestTags, tally: &mut Tally) -> Option<Candidate> {
        let cname = c.name()?;
        let (cid, cid_range) = match c.id() {
            Some(t) => (t.text().to_string(), t.text_range()),
            None => (slug(&cname.text), cname.range),
        };
        if tally.ids.contains_key(&cid) {
            self.error(
                cid_range,
                "duplicate-candidate-id",
                format!("two candidates in this contest have the id `{cid}`; give one an explicit `@id`"),
            );
        } else {
            tally.ids.insert(cid.clone(), cid_range);
        }
        let mut party = None;
        let mut incumbent = false;
        let mut write_in = false;
        for l in c.labels() {
            match l.text.as_str() {
                "incumbent" => incumbent = true,
                "write-in" => {
                    write_in = true;
                    if !tags.write_ins {
                        self.error(
                            l.range,
                            "write-in-not-allowed",
                            "a write-in candidate needs the `write-ins` tag on the contest",
                        );
                    }
                }
                other => {
                    if party.is_some() {
                        self.error(
                            l.range,
                            "multiple-party-labels",
                            "only one label may be a party; the others are `incumbent` and `write-in`",
                        );
                    }
                    party = Some(other.to_string());
                }
            }
        }
        let marker_range = c.marker_range();
        let position = match c.marker().unwrap_or(Marker::Neutral) {
            Marker::Pick => {
                tally.picks += 1;
                if tags.ranked.is_some() {
                    self.error(
                        marker_range,
                        "plus-in-ranked",
                        "in a ranked contest picks are numbered: `1.`, `2.`, ...",
                    );
                } else if tally.picks > tags.seat_count() {
                    let n = tags.seat_count();
                    self.error(
                        marker_range,
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
                if tags.ranked.is_none() {
                    self.error(
                        marker_range,
                        "rank-in-unranked",
                        "rank numbers are only for `ranked` contests; use `+`",
                    );
                }
                if n == 0 {
                    self.error(marker_range, "bad-rank", "ranks start at 1");
                }
                tally.ranks.push((n, marker_range));
                Position::Rank { rank: n }
            }
            Marker::Neutral => Position::Neutral,
            Marker::Avoid => Position::Avoid,
        };
        let note = c.note();
        Some(Candidate {
            span: Span { full: c.range(), line: c.range() },
            id: cid,
            name: cname.text,
            party,
            incumbent,
            write_in,
            designation: c.designation().map(|d| d.text),
            note: (!note.is_empty()).then(|| self.inline(&note)),
            position,
        })
    }

    /// Ranks run 1, 2, 3 with no gaps (ties share a number) and stay within
    /// `rank up to N`. Each check points at the first marker with that rank.
    fn check_ranks(&mut self, tally: &Tally, max: Option<(u32, TextRange)>) {
        let mut first: BTreeMap<u32, TextRange> = BTreeMap::new();
        for &(n, r) in tally.ranks.iter().filter(|(n, _)| *n > 0) {
            first.entry(n).or_insert(r);
        }
        for (i, (&n, &r)) in first.iter().enumerate() {
            if n != i as u32 + 1 {
                self.error(
                    r,
                    "rank-gap",
                    format!(
                        "rank {n} follows rank {i}; ranks run 1, 2, 3 with no gaps (ties share a number)"
                    ),
                );
                break;
            }
        }
        if let (Some((m, _)), Some((&top, &r))) = (max, first.iter().next_back()) {
            if top > m {
                self.error(r, "rank-exceeds-max", format!("the ballot allows at most {m} rankings"));
            }
        }
    }

    /// Spec 6.3: a contest is decided by its picks, otherwise by `vote:`,
    /// which is then required and otherwise forbidden.
    fn decision(&mut self, s: &ast::Section, picked: bool, vote: Option<(Vote, TextRange)>) -> Decision {
        match (picked, vote) {
            (true, Some((_, r))) => {
                self.error(r, "vote-with-picks", "this contest has picks, so `vote:` must be removed");
                Decision::Decided
            }
            (true, None) => Decision::Decided,
            (false, Some((Vote::None, _))) => Decision::None,
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
        let unverified = self.tags(tags, &["unverified"], &mut extensions).iter().any(|t| t == "unverified");
        let fields = self.fields(s.fields(), MEASURE_FIELDS, &mut extensions);
        let plain = |k: &str| fields.get(k).map(ast::Field::value);
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
        let unverified = self.tags(tags, &["unverified"], &mut extensions).iter().any(|t| t == "unverified");
        let fields = self.fields(s.fields(), RETENTION_FIELDS, &mut extensions);
        let vote = self.required_vote(s, &fields);
        Retention {
            span,
            id,
            name,
            description,
            unverified,
            designation: fields.get("designation").map(ast::Field::value),
            note: fields.get("note").map(|f| self.inline(&f.segments())),
            vote,
            rationale: self.rationale(s),
            extensions,
        }
    }
}
