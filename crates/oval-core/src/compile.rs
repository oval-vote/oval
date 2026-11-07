//! The canonical model of a compiled document, Section 5 of the specification.
//!
//! [`Compiled`] and the types under it mirror `guide.schema.json`, the JSON Schema that fixes the
//! canonical output. There is one struct or enum per object the schema defines, with the schema's
//! field names, and `serde` attributes that produce exactly the schema's JSON. [`crate::compile()`]
//! builds the model from a valid document; [`Compiled::to_json`] and [`Compiled::to_json_pretty`]
//! write it out.
//!
//! The model is semantic. It carries no source positions, comments, or whitespace, so two documents
//! that differ only in formatting compile to the same value. Every reference is resolved to an
//! identifier string without its leading `@`. Campaigns have no identifiers and are referred to by
//! index within their contest. Fields the schema marks as derived are computed from the document
//! as Section 5.3 defines them, so a renderer never recomputes them.
//!
//! An optional field is `None` when the author did not write it and is omitted from the JSON,
//! never written as `null`. A Markdown body is a string with its common indentation removed, line
//! terminators normalized to `\n`, and no trailing terminator.

use std::collections::HashMap;

use oval_syntax::ast::{self, BallotItem, SectionItem};
use serde::{Serialize, Serializer};

/// The canonical output of compiling one document.
///
/// The shape is fixed by the specification's `guide.schema.json`. The [`Serialize`] implementation
/// produces the schema's JSON, and [`to_json`](Self::to_json) and
/// [`to_json_pretty`](Self::to_json_pretty) produce it as a string. Root ballots are written with
/// `kind: "ballot"` like sub-ballots, so one type describes a ballot anywhere in the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Compiled {
    /// The major version of the Oval language the document declares in its version marker.
    pub oval: u32,
    /// The document-level fields, naming the guide and the election it covers.
    pub guide: Guide,
    /// Every party declared at the top level or inline, in document order.
    pub parties: Vec<Party>,
    /// Every candidate declared at the top level or inline, in document order.
    pub candidates: Vec<Candidate>,
    /// Root ballots, in presentation order.
    ///
    /// Each serializes as a [`BallotEntry::Ballot`], with `kind: "ballot"`.
    #[serde(serialize_with = "serialize_roots")]
    pub ballots: Vec<Ballot>,
    /// Derived. Every contest and measure in ballot order, with where it sits in the tree.
    ///
    /// A flat index for tables of contents and per-electorate views.
    pub questions: Vec<QuestionEntry>,
    /// Derived. Totals across the whole document.
    pub counts: Counts,
}

/// The document-level fields, naming the guide and the election it covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Guide {
    /// The guide's title, from `guide`.
    pub name: String,
    /// The election as named on the ballot, from `election`.
    pub election: String,
    /// Election day as `YYYY-MM-DD`, from `date`.
    pub date: String,
    /// The guide's introductory note, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A political party.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Party {
    /// The party's identifier, without its leading `@`.
    pub id: String,
    /// The party's name as printed on the ballot.
    pub name: String,
    /// The short form printed beside candidates, from `abbreviation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbreviation: Option<String>,
    /// The party's website, from `website`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// The author's note on the party, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Derived. Identifiers of every candidate whose party this is, in document order.
    pub candidates: Vec<String>,
}

/// A person who appears in one or more campaigns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// The candidate's identifier, without its leading `@`.
    pub id: String,
    /// The candidate's name as printed on the ballot.
    pub name: String,
    /// The occupation or title printed beside the name, from `designation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    /// Identifier of the candidate's party, from `party`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    /// The author's note on the candidate, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Derived. Identifiers of every contest in which the candidate is a campaign member, in
    /// ballot order.
    pub contests: Vec<String>,
}

/// A root ballot, or a sub-ballot for a narrower electorate.
///
/// In the canonical output a ballot always appears as a [`BallotEntry::Ballot`], which adds
/// `kind: "ballot"`; the root list is written the same way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ballot {
    /// The ballot's identifier, without its leading `@`.
    pub id: String,
    /// The jurisdiction or grouping the ballot covers.
    pub title: String,
    /// Who receives this ballot, from `eligibility`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligibility: Option<String>,
    /// The author's note on the ballot, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Derived. Identifiers of the enclosing ballots, outermost first, not including this one.
    /// Empty for a root.
    pub path: Vec<String>,
    /// Sections, contests, measures, and sub-ballots in ballot order.
    pub items: Vec<BallotEntry>,
}

/// A section, contest, measure, or sub-ballot, as the entries of a ballot.
///
/// Serializes as the inner object with a `kind` of `section`, `contest`, `measure`, or `ballot`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BallotEntry {
    /// A titled group of contests and measures.
    Section(Section),
    /// A race between campaigns for an office.
    Contest(Contest),
    /// A ballot measure, a question answered yes or no.
    Measure(Measure),
    /// A sub-ballot for a narrower electorate.
    Ballot(Ballot),
}

/// A titled group of contests and measures within a ballot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    /// The section's title as printed on the ballot.
    pub title: String,
    /// The author's note on the section, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Contests and measures in ballot order.
    pub items: Vec<Question>,
}

/// A contest or a measure, as the items of a section.
///
/// Serializes as the inner object with a `kind` of `contest` or `measure`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Question {
    /// A race between campaigns for an office.
    Contest(Contest),
    /// A ballot measure, a question answered yes or no.
    Measure(Measure),
}

/// Derived. Where a question sits in the ballot tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    /// Identifier of the innermost enclosing ballot.
    pub ballot: String,
    /// Title of the enclosing section, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Identifiers of every enclosing ballot, outermost first, ending with
    /// [`ballot`](Self::ballot).
    pub path: Vec<String>,
}

/// The voting method, from the contest header.
///
/// Serializes as an object whose `kind` is `single`, `multiple`, `ranked`, or `retention`, with
/// `seats` or `max` beside it where the variant carries one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Method {
    /// Vote for one.
    Single,
    /// Vote for up to `seats` candidates.
    Multiple {
        /// The number of candidates elected, from `multiple`. At least 2.
        seats: u32,
    },
    /// Rank candidates in order of preference.
    Ranked {
        /// The number of candidates a voter may rank, from `max`, when the contest limits it.
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
    },
    /// Yes or no on retaining one officeholder.
    Retention,
}

/// A race between campaigns for an office.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Contest {
    /// The contest's identifier, without its leading `@`.
    pub id: String,
    /// The contest as named on the ballot.
    pub title: String,
    /// A line printed under the title, from `subtitle`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// The voting method, from the contest header.
    pub method: Method,
    /// The method by which votes are counted, from `tabulation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabulation: Option<String>,
    /// The author's note on the contest, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Derived. Where the contest sits in the ballot tree.
    pub location: Location,
    /// The campaigns, in ballot order.
    ///
    /// A campaign is referred to by its index in this list. [`Campaign::index`] and the
    /// `support_campaigns` and `oppose_campaigns` fields of [`ContestRecommendation`] use these
    /// indices.
    pub campaigns: Vec<Campaign>,
    /// The author's recommendation on the contest.
    pub recommendation: ContestRecommendation,
}

/// One or more candidates running together for a contest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Campaign {
    /// Derived. Position in the contest's campaigns list.
    pub index: usize,
    /// Derived. The members' names joined with `" / "`, head of ticket first. For a single
    /// candidate, their name.
    pub label: String,
    /// Derived. Identifier of the first member. That member is the head of the ticket, and the
    /// recommendation names the campaign by this identifier.
    pub head: String,
    /// Derived. The party shared by every member, when all members have the same one.
    ///
    /// `None` otherwise, including when any member has no party.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    /// The campaign's candidates, head of ticket first.
    pub members: Vec<Member>,
    /// Whether the campaign currently occupies the office, from the `incumbent` flag.
    pub incumbent: bool,
    /// Whether the campaign has ended but remains on the ballot, from the `withdrawn` flag.
    pub withdrawn: bool,
    /// The campaign's website, from `website`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// The author's note on the campaign, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Derived. What the contest's recommendation says about this campaign.
    pub stance: Stance,
}

/// A candidate's place on a campaign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Member {
    /// The candidate's identifier, without its leading `@`.
    pub candidate: String,
    /// The member's role on the ticket, from `as`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The author's note on this member, from the member's `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Derived. What a contest's recommendation says about one campaign.
///
/// Serializes as an object whose `position` is `support`, `oppose`, or `none`, with `tier` beside
/// `support`. In a no-position recommendation every campaign is [`Stance::None`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "position", rename_all = "snake_case")]
pub enum Stance {
    /// The campaign is named on a `support` line.
    Support {
        /// Zero-based index of the support tier the campaign appears in; 0 is the most preferred.
        tier: usize,
    },
    /// The campaign is named on the `oppose` line.
    Oppose,
    /// The recommendation does not name the campaign.
    None,
}

/// The author's recommendation on a contest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContestRecommendation {
    /// Whether the author takes no position, from `recommend no-position`.
    pub no_position: bool,
    /// Tiers of candidate identifiers as written, most preferred tier first. Empty when
    /// [`no_position`](Self::no_position).
    pub support: Vec<Vec<String>>,
    /// Candidate identifiers as written. Empty when [`no_position`](Self::no_position).
    pub oppose: Vec<String>,
    /// Derived. The tiers of [`support`](Self::support) resolved to campaign indices.
    pub support_campaigns: Vec<Vec<usize>>,
    /// Derived. The opposed campaigns as indices.
    pub oppose_campaigns: Vec<usize>,
    /// The author's reasoning, from `justification`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
    /// The author's note on the recommendation, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A ballot measure, a question answered yes or no.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Measure {
    /// The measure's identifier, without its leading `@`.
    pub id: String,
    /// The measure as named on the ballot.
    pub title: String,
    /// A line printed under the title, from `subtitle`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// The question as printed on the ballot, from `description`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The effect of a yes vote, from `if_yes`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_yes: Option<String>,
    /// The effect of a no vote, from `if_no`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_no: Option<String>,
    /// The way the measure reached the ballot, from `placement`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
    /// The threshold the measure needs to pass, from `tabulation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabulation: Option<String>,
    /// The measure's fiscal impact, from `fiscal_impact`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiscal_impact: Option<String>,
    /// Organizations supporting the measure, from `supporters`.
    ///
    /// `None` when not researched, and then omitted from the JSON; empty when researched and none
    /// known, which `supporters none` writes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supporters: Option<Vec<String>>,
    /// Organizations opposing the measure, from `opponents`.
    ///
    /// `None` when not researched, and then omitted from the JSON; empty when researched and none
    /// known, which `opponents none` writes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponents: Option<Vec<String>>,
    /// The author's note on the measure, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Derived. Where the measure sits in the ballot tree.
    pub location: Location,
    /// The author's recommendation on the measure.
    pub recommendation: MeasureRecommendation,
}

/// The author's recommendation on a measure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeasureRecommendation {
    /// The recommended vote, from `recommend`.
    pub position: Position,
    /// The author's reasoning, from `justification`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
    /// The author's note on the recommendation, from `note`, as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The recommended vote on a measure.
///
/// Serializes as `yes`, `no`, or `no-position`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    /// Vote yes.
    Yes,
    /// Vote no.
    No,
    /// The author takes no position.
    NoPosition,
}

/// Derived. One contest or measure in the document's flat index of questions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuestionEntry {
    /// Whether the question is a contest or a measure.
    pub kind: QuestionKind,
    /// The question's identifier, without its leading `@`.
    pub id: String,
    /// The question's title.
    pub title: String,
    /// The question's position in the ballot tree.
    pub location: Location,
}

/// The two kinds of question.
///
/// Serializes as `contest` or `measure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    /// A race between campaigns for an office.
    Contest,
    /// A ballot measure, a question answered yes or no.
    Measure,
}

/// Derived. Totals across the whole document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// Every ballot, roots and sub-ballots alike.
    pub ballots: usize,
    /// Every section.
    pub sections: usize,
    /// Every contest.
    pub contests: usize,
    /// Every measure.
    pub measures: usize,
    /// Every campaign.
    pub campaigns: usize,
    /// Every candidate.
    pub candidates: usize,
    /// Every party.
    pub parties: usize,
}

fn serialize_roots<S: Serializer>(ballots: &[Ballot], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(ballots.iter().cloned().map(BallotEntry::Ballot))
}

impl Compiled {
    /// Serializes the model to its canonical JSON, compact.
    ///
    /// The result validates against the specification's `guide.schema.json`. `oval compile` prints
    /// this form.
    ///
    /// # Errors
    ///
    /// Returns the [`serde_json::Error`] when the value cannot be written.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Serializes the model to its canonical JSON, indented for reading.
    ///
    /// The value is the same as [`to_json`](Self::to_json) produces; only the whitespace differs.
    /// `oval compile --pretty` prints this form.
    ///
    /// # Errors
    ///
    /// Returns the [`serde_json::Error`] when the value cannot be written.
    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

/// Compiles a validated document, or `None` when a part the model requires is missing.
///
/// The caller has already checked the document, so `None` only arises from a tree the validator
/// rejected; [`crate::compile()`] turns it into the report.
pub(crate) fn compile(document: &ast::Document) -> Option<Compiled> {
    let mut compiler = Compiler {
        parties: Vec::new(),
        candidates: Vec::new(),
        party_candidates: HashMap::new(),
        candidate_contests: HashMap::new(),
        questions: Vec::new(),
        counts: Counts::default(),
    };
    compiler.declarations(document)?;
    let ballots = document
        .ballots()
        .map(|ballot| compiler.ballot(&ballot, &[]))
        .collect::<Option<Vec<_>>>()?;
    compiler.finish(document, ballots)
}

/// The state of one compilation, which walks the tree and resolves references as it goes.
struct Compiler {
    parties: Vec<Party>,
    candidates: Vec<Candidate>,
    party_candidates: HashMap<String, Vec<String>>,
    candidate_contests: HashMap<String, Vec<String>>,
    questions: Vec<QuestionEntry>,
    counts: Counts,
}

impl Compiler {
    fn declarations(&mut self, document: &ast::Document) -> Option<()> {
        for party in document.parties() {
            let party = Party {
                id: party.id()?.name(),
                name: party.name()?,
                abbreviation: party.abbreviation().and_then(|f| f.value()),
                website: party.website().and_then(|f| f.value()),
                note: party.note().and_then(|f| f.value()),
                candidates: Vec::new(),
            };
            self.parties.push(party);
        }
        for candidate in document.candidates() {
            let party = match candidate.party() {
                Some(reference) => Some(self.party_reference(&reference)?),
                None => None,
            };
            let candidate = Candidate {
                id: candidate.id()?.name(),
                name: candidate.name()?,
                designation: candidate.designation().and_then(|f| f.value()),
                party,
                note: candidate.note().and_then(|f| f.value()),
                contests: Vec::new(),
            };
            self.candidates.push(candidate);
        }
        Some(())
    }

    fn party_reference(&mut self, reference: &ast::PartyReference) -> Option<String> {
        let id = reference.id()?.name();
        if reference.is_declaration() {
            self.parties.push(Party {
                id: id.clone(),
                name: reference.name()?,
                abbreviation: reference.abbreviation().and_then(|f| f.value()),
                website: reference.website().and_then(|f| f.value()),
                note: reference.note().and_then(|f| f.value()),
                candidates: Vec::new(),
            });
        }
        Some(id)
    }

    fn ballot(&mut self, ballot: &ast::Ballot, path: &[String]) -> Option<Ballot> {
        self.counts.ballots += 1;
        let id = ballot.id()?.name();
        let mut inner_path = path.to_vec();
        inner_path.push(id.clone());
        let mut items = Vec::new();
        for item in ballot.items() {
            items.push(match item {
                BallotItem::Section(section) => {
                    self.counts.sections += 1;
                    let mut questions = Vec::new();
                    let title = section.title()?;
                    for item in section.items() {
                        let location = Location {
                            ballot: id.clone(),
                            section: Some(title.clone()),
                            path: inner_path.clone(),
                        };
                        questions.push(match item {
                            SectionItem::Contest(c) => {
                                Question::Contest(self.contest(&c, location)?)
                            }
                            SectionItem::Measure(m) => {
                                Question::Measure(self.measure(&m, location)?)
                            }
                        });
                    }
                    BallotEntry::Section(Section {
                        title,
                        note: section.note().and_then(|f| f.value()),
                        items: questions,
                    })
                }
                BallotItem::Contest(contest) => {
                    let location = Location {
                        ballot: id.clone(),
                        section: None,
                        path: inner_path.clone(),
                    };
                    BallotEntry::Contest(self.contest(&contest, location)?)
                }
                BallotItem::Measure(measure) => {
                    let location = Location {
                        ballot: id.clone(),
                        section: None,
                        path: inner_path.clone(),
                    };
                    BallotEntry::Measure(self.measure(&measure, location)?)
                }
                BallotItem::Ballot(sub) => BallotEntry::Ballot(self.ballot(&sub, &inner_path)?),
            });
        }
        Some(Ballot {
            id,
            title: ballot.title()?,
            eligibility: ballot.eligibility().and_then(|f| f.value()),
            note: ballot.note().and_then(|f| f.value()),
            path: path.to_vec(),
            items,
        })
    }

    fn contest(&mut self, contest: &ast::Contest, location: Location) -> Option<Contest> {
        self.counts.contests += 1;
        let id = contest.id()?.name();
        let title = contest.title()?;
        let method = match contest.kind()? {
            ast::ContestKind::Single => Method::Single,
            ast::ContestKind::Multiple { seats } => Method::Multiple { seats: seats? },
            ast::ContestKind::Ranked { max } => Method::Ranked { max },
            ast::ContestKind::Retention => Method::Retention,
        };
        let mut campaigns = Vec::new();
        let mut campaign_of: HashMap<String, usize> = HashMap::new();
        for (index, campaign) in contest.campaigns().enumerate() {
            campaigns.push(self.campaign(&id, index, &campaign, &mut campaign_of)?);
        }
        let recommendation = Self::recommendation(contest, &campaign_of, &mut campaigns)?;
        self.questions.push(QuestionEntry {
            kind: QuestionKind::Contest,
            id: id.clone(),
            title: title.clone(),
            location: location.clone(),
        });
        Some(Contest {
            id,
            title,
            subtitle: contest.subtitle().and_then(|f| f.value()),
            method,
            tabulation: contest.tabulation().and_then(|f| f.value()),
            note: contest.note().and_then(|f| f.value()),
            location,
            campaigns,
            recommendation,
        })
    }

    fn campaign(
        &mut self,
        contest_id: &str,
        index: usize,
        campaign: &ast::Campaign,
        campaign_of: &mut HashMap<String, usize>,
    ) -> Option<Campaign> {
        self.counts.campaigns += 1;
        let mut members = Vec::new();
        let mut names = Vec::new();
        let mut parties: Vec<Option<String>> = Vec::new();
        for member in campaign.members() {
            let candidate_id = member.id()?.name();
            campaign_of.entry(candidate_id.clone()).or_insert(index);
            self.candidate_contests
                .entry(candidate_id.clone())
                .or_default()
                .push(contest_id.to_owned());
            if member.is_declaration() {
                let party = match member.party().or_else(|| campaign.party()) {
                    Some(reference) => Some(self.party_reference(&reference)?),
                    None => None,
                };
                self.candidates.push(Candidate {
                    id: candidate_id.clone(),
                    name: member.name()?,
                    designation: member
                        .designation()
                        .or_else(|| campaign.designation())
                        .and_then(|f| f.value()),
                    party,
                    note: member.note().and_then(|f| f.value()),
                    contests: Vec::new(),
                });
            }
            let (name, party) = self.candidate_name_and_party(&candidate_id)?;
            names.push(name);
            parties.push(party);
            members.push(Member {
                candidate: candidate_id,
                role: member.role(),
                note: member.note().and_then(|f| f.value()),
            });
        }
        let head = members.first()?.candidate.clone();
        let party = match parties.first() {
            Some(Some(first)) if parties.iter().all(|p| p.as_deref() == Some(first)) => {
                Some(first.clone())
            }
            _ => None,
        };
        Some(Campaign {
            index,
            label: names.join(" / "),
            head,
            party,
            members,
            incumbent: campaign.is_incumbent(),
            withdrawn: campaign.is_withdrawn(),
            website: campaign.website().and_then(|f| f.value()),
            note: campaign.note().and_then(|f| f.value()),
            stance: Stance::None,
        })
    }

    fn recommendation(
        contest: &ast::Contest,
        campaign_of: &HashMap<String, usize>,
        campaigns: &mut [Campaign],
    ) -> Option<ContestRecommendation> {
        let recommendation = contest.recommendations().next()?;
        let support: Vec<Vec<String>> = recommendation
            .support()
            .map(|tier| tier.candidates().map(|c| c.name()).collect())
            .collect();
        let oppose: Vec<String> = recommendation
            .oppose()
            .flat_map(|o| o.candidates().map(|c| c.name()).collect::<Vec<_>>())
            .collect();
        let resolve = |names: &[String]| -> Option<Vec<usize>> {
            names.iter().map(|n| campaign_of.get(n).copied()).collect()
        };
        let support_campaigns: Vec<Vec<usize>> = support
            .iter()
            .map(|tier| resolve(tier))
            .collect::<Option<_>>()?;
        let oppose_campaigns = resolve(&oppose)?;
        for (tier, indices) in support_campaigns.iter().enumerate() {
            for &index in indices {
                campaigns[index].stance = Stance::Support { tier };
            }
        }
        for &index in &oppose_campaigns {
            campaigns[index].stance = Stance::Oppose;
        }
        Some(ContestRecommendation {
            no_position: recommendation.is_no_position(),
            support,
            oppose,
            support_campaigns,
            oppose_campaigns,
            justification: recommendation.justification().and_then(|f| f.value()),
            note: recommendation.note().and_then(|f| f.value()),
        })
    }

    fn candidate_name_and_party(&self, id: &str) -> Option<(String, Option<String>)> {
        self.candidates
            .iter()
            .find(|c| c.id == id)
            .map(|c| (c.name.clone(), c.party.clone()))
    }

    fn measure(&mut self, measure: &ast::Measure, location: Location) -> Option<Measure> {
        self.counts.measures += 1;
        let id = measure.id()?.name();
        let title = measure.title()?;
        let recommendation = measure.recommendations().next()?;
        let position = match recommendation.position()? {
            ast::Position::Yes => Position::Yes,
            ast::Position::No => Position::No,
            ast::Position::NoPosition => Position::NoPosition,
        };
        self.questions.push(QuestionEntry {
            kind: QuestionKind::Measure,
            id: id.clone(),
            title: title.clone(),
            location: location.clone(),
        });
        Some(Measure {
            id,
            title,
            subtitle: measure.subtitle().and_then(|f| f.value()),
            description: measure.description().and_then(|f| f.value()),
            if_yes: measure.if_yes().and_then(|f| f.value()),
            if_no: measure.if_no().and_then(|f| f.value()),
            placement: measure.placement().and_then(|f| f.value()),
            tabulation: measure.tabulation().and_then(|f| f.value()),
            fiscal_impact: measure.fiscal_impact().and_then(|f| f.value()),
            supporters: measure.supporters().and_then(|f| f.value()),
            opponents: measure.opponents().and_then(|f| f.value()),
            note: measure.note().and_then(|f| f.value()),
            location,
            recommendation: MeasureRecommendation {
                position,
                justification: recommendation.justification().and_then(|f| f.value()),
                note: recommendation.note().and_then(|f| f.value()),
            },
        })
    }

    fn finish(mut self, document: &ast::Document, ballots: Vec<Ballot>) -> Option<Compiled> {
        for candidate in &mut self.candidates {
            if let Some(contests) = self.candidate_contests.remove(&candidate.id) {
                candidate.contests = contests;
            }
            if let Some(party) = &candidate.party {
                self.party_candidates
                    .entry(party.clone())
                    .or_default()
                    .push(candidate.id.clone());
            }
        }
        for party in &mut self.parties {
            if let Some(candidates) = self.party_candidates.remove(&party.id) {
                party.candidates = candidates;
            }
        }
        self.counts.candidates = self.candidates.len();
        self.counts.parties = self.parties.len();
        let date = document.date()?.value()?;
        Some(Compiled {
            oval: document.version()?.value()?,
            guide: Guide {
                name: document.guide()?.value()?,
                election: document.election()?.value()?,
                date: format!("{:04}-{:02}-{:02}", date.year, date.month, date.day),
                note: document.note().and_then(|f| f.value()),
            },
            parties: self.parties,
            candidates: self.candidates,
            ballots,
            questions: self.questions,
            counts: self.counts,
        })
    }
}
