//! The canonical compiled form of a document: the data model with references resolved.

use std::collections::HashMap;

use oval_syntax::ast::{self, BallotItem, SectionItem};
use serde::{Serialize, Serializer};

/// A compiled guide. The shape is fixed by the specification's `guide.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Compiled {
    /// The specification version the document was compiled under.
    pub oval: String,
    /// Guide-level fields.
    pub guide: Guide,
    /// Every party, in document order.
    pub parties: Vec<Party>,
    /// Every candidate, in document order.
    pub candidates: Vec<Candidate>,
    /// Root ballots, in presentation order.
    #[serde(serialize_with = "serialize_roots")]
    pub ballots: Vec<Ballot>,
    /// Every contest and measure in ballot order, with its location.
    pub questions: Vec<QuestionEntry>,
    /// Totals across the document.
    pub counts: Counts,
}

/// Guide-level fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Guide {
    /// The guide's own name.
    pub name: String,
    /// The election's official name.
    pub election: String,
    /// Election day, `YYYY-MM-DD`.
    pub date: String,
    /// Guide-level commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A party.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Party {
    /// The identifier without `@`.
    pub id: String,
    /// The name as printed on the ballot.
    pub name: String,
    /// Short form for compact rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbreviation: Option<String>,
    /// The party's website.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// Author commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Identifiers of every candidate whose party this is, in document order.
    pub candidates: Vec<String>,
}

/// A candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// The identifier without `@`.
    pub id: String,
    /// The name as printed on the ballot.
    pub name: String,
    /// The ballot designation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    /// The party's identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    /// Author commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Identifiers of every contest in which the candidate is a campaign member, in ballot order.
    pub contests: Vec<String>,
}

/// A ballot: a node in the electorate tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ballot {
    /// The identifier without `@`.
    pub id: String,
    /// The jurisdiction or grouping this ballot covers.
    pub title: String,
    /// Who votes on this ballot, when the title does not say.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligibility: Option<String>,
    /// Author commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Identifiers of the enclosing ballots, outermost first. Empty for a root.
    pub path: Vec<String>,
    /// Contents in ballot order.
    pub items: Vec<BallotEntry>,
}

/// One entry in a ballot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BallotEntry {
    /// A titled group of questions.
    Section(Section),
    /// A contest.
    Contest(Contest),
    /// A measure.
    Measure(Measure),
    /// A sub-ballot.
    Ballot(Ballot),
}

/// A titled group of questions within a ballot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    /// The heading.
    pub title: String,
    /// Author commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Questions in ballot order.
    pub items: Vec<Question>,
}

/// A contest or a measure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Question {
    /// A contest.
    Contest(Contest),
    /// A measure.
    Measure(Measure),
}

/// Where a question sits in the ballot tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    /// The innermost enclosing ballot.
    pub ballot: String,
    /// The enclosing section's title, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Every enclosing ballot, outermost first, ending with `ballot`.
    pub path: Vec<String>,
}

/// A contest's voting method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Method {
    /// Vote for one.
    Single,
    /// Vote for up to `seats`.
    Multiple {
        /// Seats to fill.
        seats: u32,
    },
    /// Rank choices, optionally capped.
    Ranked {
        /// How many may be ranked, when capped.
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
    },
    /// Yes or no on one officeholder.
    Retention,
}

/// A contest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Contest {
    /// The identifier without `@`.
    pub id: String,
    /// The contest as named on the ballot.
    pub title: String,
    /// Secondary ballot text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// The voting method.
    pub method: Method,
    /// How the winner is decided, in plain words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabulation: Option<String>,
    /// Author commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Where the contest sits.
    pub location: Location,
    /// Campaigns in ballot order.
    pub campaigns: Vec<Campaign>,
    /// The author's advice.
    pub recommendation: ContestRecommendation,
}

/// A campaign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Campaign {
    /// Position in the contest's campaigns.
    pub index: usize,
    /// The members' names joined with `" / "`.
    pub label: String,
    /// The first member's identifier.
    pub head: String,
    /// The party every member shares, when they all share one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    /// The members, head first.
    pub members: Vec<Member>,
    /// Whether the campaign holds the office.
    pub incumbent: bool,
    /// Whether the campaign has withdrawn.
    pub withdrawn: bool,
    /// The campaign's website.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// Author commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// What the recommendation says about this campaign.
    pub stance: Stance,
}

/// A member of a campaign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Member {
    /// The candidate's identifier.
    pub candidate: String,
    /// The role on the ticket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Author commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What a recommendation says about one campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "position", rename_all = "snake_case")]
pub enum Stance {
    /// Supported, in the given zero-based tier.
    Support {
        /// The tier.
        tier: usize,
    },
    /// Opposed.
    Oppose,
    /// Not mentioned.
    None,
}

/// A contest's recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContestRecommendation {
    /// Whether the author takes no position.
    pub no_position: bool,
    /// Tiers of candidate identifiers as written, most preferred first.
    pub support: Vec<Vec<String>>,
    /// Opposed candidate identifiers as written.
    pub oppose: Vec<String>,
    /// The tiers resolved to campaign indices.
    pub support_campaigns: Vec<Vec<usize>>,
    /// The opposed campaigns as indices.
    pub oppose_campaigns: Vec<usize>,
    /// The argument for the advice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
    /// Additional content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A measure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Measure {
    /// The identifier without `@`.
    pub id: String,
    /// The measure as named on the ballot.
    pub title: String,
    /// Secondary ballot text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Official description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Official statement of a yes vote's effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_yes: Option<String>,
    /// Official statement of a no vote's effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_no: Option<String>,
    /// How the measure reached the ballot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
    /// The passage rule, in plain words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabulation: Option<String>,
    /// A one-line fiscal summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiscal_impact: Option<String>,
    /// Notable supporters; `None` when not researched, empty when none known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supporters: Option<Vec<String>>,
    /// Notable opponents; `None` when not researched, empty when none known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponents: Option<Vec<String>>,
    /// Author commentary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Where the measure sits.
    pub location: Location,
    /// The author's advice.
    pub recommendation: MeasureRecommendation,
}

/// A measure's recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeasureRecommendation {
    /// How the author advises voting.
    pub position: Position,
    /// The argument for the advice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
    /// Additional content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// How the author advises voting on a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    /// Vote yes.
    Yes,
    /// Vote no.
    No,
    /// No position.
    NoPosition,
}

/// One row of the flat question index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuestionEntry {
    /// Whether this is a contest or a measure.
    pub kind: QuestionKind,
    /// The identifier without `@`.
    pub id: String,
    /// The title.
    pub title: String,
    /// Where it sits.
    pub location: Location,
}

/// The two kinds of question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    /// A contest.
    Contest,
    /// A measure.
    Measure,
}

/// Totals across the document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// All ballots, including nested ones.
    pub ballots: usize,
    /// Sections.
    pub sections: usize,
    /// Contests.
    pub contests: usize,
    /// Measures.
    pub measures: usize,
    /// Campaigns.
    pub campaigns: usize,
    /// Candidates.
    pub candidates: usize,
    /// Parties.
    pub parties: usize,
}

fn serialize_roots<S: Serializer>(ballots: &[Ballot], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(ballots.iter().cloned().map(BallotEntry::Ballot))
}

impl Compiled {
    /// The canonical JSON form, compact.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// The canonical JSON form, indented for reading.
    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

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
                id: party.id()?.name().to_owned(),
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
                id: candidate.id()?.name().to_owned(),
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
        let id = reference.id()?.name().to_owned();
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
        let id = ballot.id()?.name().to_owned();
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
        let id = contest.id()?.name().to_owned();
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
            let candidate_id = member.id()?.name().to_owned();
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
            .map(|tier| tier.candidates().map(|c| c.name().to_owned()).collect())
            .collect();
        let oppose: Vec<String> = recommendation
            .oppose()
            .flat_map(|o| {
                o.candidates()
                    .map(|c| c.name().to_owned())
                    .collect::<Vec<_>>()
            })
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
        let id = measure.id()?.name().to_owned();
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
            oval: oval_syntax::SPEC_VERSION.to_owned(),
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
