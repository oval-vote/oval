//! The canonical output of compiling a valid document.
//!
//! [`Compiled`] and the types it holds have one field for each key of the specification's schema,
//! named as the schema names it, so serializing them with [`serde`] produces the canonical JSON. An
//! optional field the author did not write is `None` and is left out of the JSON rather than
//! written as `null`. A Markdown body is a string with the indentation its lines share removed,
//! each line terminator written as `\n`, and no terminator after the last line.
//!
//! ```
//! let text = "oval 1\nguide \"Franklin Voter Guide\"\nelection \"General Election\"\n\
//!     date 2026-11-03\nballot @city \"City\"\n  measure @a \"Measure A\"\n    recommend yes\n";
//! let compiled = oval_core::check(text)?.compile().expect("the document is valid");
//! assert_eq!(compiled.guide.name, "Franklin Voter Guide");
//! assert_eq!(compiled.guide.date, "2026-11-03");
//! # Ok::<(), oval_core::InputRejection>(())
//! ```

use serde::Serialize;

/// The canonical output of compiling a valid document, the guide it describes with every reference
/// resolved.
///
/// Get one from [`Report::compile`](crate::Report::compile), and write it as JSON with
/// [`to_json`](Self::to_json) or [`to_json_pretty`](Self::to_json_pretty).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Compiled {
    /// The major version of the language the document declares in its version marker.
    pub oval: u64,
    /// The document-level fields, which name the guide and the election it covers.
    pub guide: Guide,
    /// Every party, declared at the top level or inline, in document order.
    pub parties: Vec<Party>,
    /// Every candidate, declared at the top level or inline, in document order.
    pub candidates: Vec<Candidate>,
    /// The root ballots, in ballot order.
    pub ballots: Vec<Ballot>,
    /// Every contest and measure in ballot order, with where each sits in the tree.
    pub questions: Vec<QuestionEntry>,
    /// Totals across the whole document.
    pub counts: Counts,
}

impl Compiled {
    /// Returns the canonical JSON, with no whitespace between its tokens.
    ///
    /// # Panics
    ///
    /// Never, since every value in the output can be written as JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("the output can be written as JSON")
    }

    /// Returns the canonical JSON, indented by two spaces for people to read.
    ///
    /// The value is the one [`to_json`](Self::to_json) returns, and only the whitespace differs.
    ///
    /// # Panics
    ///
    /// Never, since every value in the output can be written as JSON.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("the output can be written as JSON")
    }
}

/// The document-level fields, which name the guide and the election it covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Guide {
    /// The guide's title, from `guide`.
    pub name: String,
    /// The election as it is named on the ballot, from `election`.
    pub election: String,
    /// Election day, from `date`, written as `YYYY-MM-DD`.
    pub date: String,
    /// The guide's introductory note in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A political party.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Party {
    /// The party's identifier, without its `@`.
    pub id: String,
    /// The party's name as it is printed on the ballot.
    pub name: String,
    /// The short form printed beside the party's candidates, from `abbreviation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbreviation: Option<String>,
    /// The party's website, from `website`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// The author's note on the party in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The identifiers of every candidate whose party this is, in document order.
    pub candidates: Vec<String>,
    /// The identifiers of every contest with a campaign the party is a member of, in ballot order.
    pub contests: Vec<String>,
}

/// A person who appears in one or more campaigns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// The candidate's identifier, without its `@`.
    pub id: String,
    /// The candidate's name as it is printed on the ballot.
    pub name: String,
    /// The occupation or title printed beside the name, from `designation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    /// The identifier of the candidate's party, from `party`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    /// The author's note on the candidate in Markdown, from `note`.
    ///
    /// The note under a one-line campaign belongs to the campaign, so a candidate declared on a
    /// `campaign` line has no note of their own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The identifiers of every contest with a campaign the candidate is a member of, in ballot
    /// order.
    pub contests: Vec<String>,
}

/// A ballot, either a root ballot or a sub-ballot for a narrower electorate.
///
/// Its JSON has a `kind` of `"ballot"` wherever it appears, so root ballots and sub-ballots share
/// one shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename = "ballot")]
pub struct Ballot {
    /// The ballot's identifier, without its `@`.
    pub id: String,
    /// The jurisdiction or grouping the ballot covers.
    pub title: String,
    /// Who receives the ballot, from `eligibility`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligibility: Option<String>,
    /// The author's note on the ballot in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The identifiers of the ballots that enclose this one, outermost first, which is empty for a
    /// root ballot.
    pub path: Vec<String>,
    /// The ballot's sections, contests, measures, and sub-ballots, in ballot order.
    pub items: Vec<BallotItem>,
}

/// One item of a ballot.
///
/// Each item's JSON has a `kind` naming what it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum BallotItem {
    /// A titled group of questions.
    Section(Section),
    /// A race between campaigns for an office.
    Contest(Contest),
    /// A question answered yes or no.
    Measure(Measure),
    /// A sub-ballot.
    Ballot(Ballot),
}

/// A titled group of contests and measures within a ballot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename = "section")]
pub struct Section {
    /// The section's title as it is printed on the ballot.
    pub title: String,
    /// The author's note on the section in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The section's contests and measures, in ballot order.
    pub items: Vec<SectionItem>,
}

/// One item of a section.
///
/// Each item's JSON has a `kind` naming what it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SectionItem {
    /// A race between campaigns for an office.
    Contest(Contest),
    /// A question answered yes or no.
    Measure(Measure),
}

/// A race between campaigns for an office.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename = "contest")]
pub struct Contest {
    /// The contest's identifier, without its `@`.
    pub id: String,
    /// The contest as it is named on the ballot.
    pub title: String,
    /// A line printed under the title, from `subtitle`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// The voting method, from the contest's header.
    pub method: Method,
    /// How votes are counted, from `tabulation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabulation: Option<String>,
    /// The author's note on the contest in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Where the contest sits in the ballot tree.
    pub location: Location,
    /// Whether a voter must answer the contest, from the nearest `required` or `optional` on the
    /// contest, its section, or its ballots, and `false` if there is none.
    pub required: bool,
    /// Whether a voter may write in a choice, from the nearest `extensible` or `exhaustive` on the
    /// contest, its section, or its ballots, and `false` if there is none or the contest is a
    /// retention contest.
    pub extensible: bool,
    /// The contest's campaigns in ballot order.
    pub campaigns: Vec<Campaign>,
    /// The author's recommendation on the contest.
    pub recommendation: ContestRecommendation,
}

/// A contest's voting method, from its header, with each number the header leaves out filled in
/// with its default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Method {
    /// A vote for one choice.
    Single {
        /// How many candidates are elected, from `seats`, or 1.
        seats: u64,
    },
    /// A vote for up to a number of choices.
    Multiple {
        /// How many choices a voter may mark, from `multiple`.
        votes: u64,
        /// How many candidates are elected, from `seats`, or the number of votes.
        seats: u64,
        /// How many choices a ballot that marks the contest must mark, from `min`, or 1.
        min: u64,
    },
    /// A ranking of choices in order of preference.
    Ranked {
        /// How many candidates are elected, from `seats`, or 1.
        seats: u64,
        /// How many choices a voter may rank, from `max`, or `None` when there is no limit.
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<u64>,
        /// How many choices a ballot that ranks the contest must rank, from `min`, or 1, or the
        /// number of campaigns when every choice must be ranked.
        min: u64,
        /// Whether every printed choice must be ranked, from `all`.
        complete: bool,
    },
    /// A yes or no on one candidate.
    Retention,
}

/// One or more candidates, or one or more parties, running together for a contest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Campaign {
    /// The campaign's position in its contest's campaigns, which is how the rest of the output
    /// refers to it, since a campaign has no identifier.
    pub index: usize,
    /// Whether the members are candidates or parties.
    pub kind: CampaignKind,
    /// The members' names joined with `" / "`, head first.
    pub label: String,
    /// The identifier of the first member, the head of the ticket or the lead party.
    pub head: String,
    /// The party every member of a candidate campaign belongs to, or `None` if they don't all
    /// belong to the same one or the campaign is a party campaign.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<String>,
    /// The campaign's candidates or parties, head first.
    pub members: Vec<Member>,
    /// Whether the campaign currently holds the office, from the `incumbent` flag.
    pub incumbent: bool,
    /// Whether the campaign has ended but is still on the ballot, from the `withdrawn` flag.
    pub withdrawn: bool,
    /// The campaign's website, from `website`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// The author's note on the campaign in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// What the contest's recommendation says about the campaign.
    pub stance: Stance,
}

/// Whether a campaign's members are candidates or parties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CampaignKind {
    /// A candidate campaign, written `campaign`.
    Candidates,
    /// A party campaign, written `campaign party`.
    Parties,
}

/// A candidate's or a party's place in a campaign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Member {
    /// A member of a candidate campaign.
    Candidate {
        /// The candidate's identifier.
        candidate: String,
        /// The member's role on the ticket, from `as`.
        #[serde(skip_serializing_if = "Option::is_none")]
        role: Option<String>,
        /// The author's note on the member in Markdown, from the `note` in the member's own
        /// block.
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    /// A member of a party campaign.
    Party {
        /// The party's identifier.
        party: String,
    },
}

/// What a contest's recommendation says about one of its campaigns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "position", rename_all = "lowercase")]
pub enum Stance {
    /// The campaign is in a `support` line.
    Support {
        /// The zero-based index of that `support` line among the recommendation's tiers.
        tier: usize,
    },
    /// The campaign is in the `oppose` line.
    Oppose,
    /// The recommendation doesn't name the campaign.
    None,
}

/// The author's recommendation on a contest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContestRecommendation {
    /// Whether the recommendation takes a stance or is `no-position` or `abstain`.
    pub position: ContestPosition,
    /// The `support` lines, most preferred first, each a tier of interchangeable choices.
    ///
    /// It's empty unless the position is a stance.
    pub support: Vec<Vec<Choice>>,
    /// The choices in the `oppose` line, as written.
    ///
    /// It's empty unless the position is a stance.
    pub oppose: Vec<Choice>,
    /// The author's reasoning in Markdown, from `justification`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
    /// The author's note on the recommendation in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The kind of recommendation on a contest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContestPosition {
    /// The recommendation has `support` or `oppose` lines.
    Stance,
    /// The author takes no position, from `recommend no-position`.
    NoPosition,
    /// The author advises leaving the contest blank, from `recommend abstain`.
    Abstain,
}

/// One choice in a recommendation, a campaign printed on the ballot or a write-in.
///
/// A reference may name any member of a campaign, and it's written as the campaign's head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Choice {
    /// A candidate campaign.
    Candidate {
        /// The identifier of the campaign's head.
        candidate: String,
        /// The campaign's index in its contest's campaigns.
        campaign: usize,
    },
    /// A party campaign.
    Party {
        /// The identifier of the campaign's lead party.
        party: String,
        /// The campaign's index in its contest's campaigns.
        campaign: usize,
    },
    /// A choice the voter writes on the ballot by hand.
    WriteIn {
        /// The name to write in, or `None` for a bare `write-in`.
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

/// A ballot measure, a question answered yes or no.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename = "measure")]
pub struct Measure {
    /// The measure's identifier, without its `@`.
    pub id: String,
    /// The measure as it is named on the ballot.
    pub title: String,
    /// A line printed under the title, from `subtitle`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// The question as it is printed on the ballot in Markdown, from `description`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What a yes vote does in Markdown, from `if_yes`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_yes: Option<String>,
    /// What a no vote does in Markdown, from `if_no`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_no: Option<String>,
    /// How the measure reached the ballot, from `placement`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
    /// What it takes to pass, from `tabulation`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabulation: Option<String>,
    /// The measure's fiscal impact, from `fiscal_impact`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiscal_impact: Option<String>,
    /// The organizations supporting the measure, from `supporters`.
    ///
    /// It is `None` when they were not researched, and empty when the author wrote `none`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supporters: Option<Vec<String>>,
    /// The organizations opposing the measure, from `opponents`.
    ///
    /// It is `None` when they were not researched, and empty when the author wrote `none`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponents: Option<Vec<String>>,
    /// The author's note on the measure in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Where the measure sits in the ballot tree.
    pub location: Location,
    /// Whether a voter must answer the measure, from the nearest `required` or `optional` on the
    /// measure, its section, or its ballots, and `false` if there is none.
    pub required: bool,
    /// The author's recommendation on the measure.
    pub recommendation: MeasureRecommendation,
}

/// The author's recommendation on a measure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeasureRecommendation {
    /// The recommended vote, from `recommend`.
    pub position: MeasurePosition,
    /// The author's reasoning in Markdown, from `justification`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
    /// The author's note on the recommendation in Markdown, from `note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The vote recommended on a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeasurePosition {
    /// `yes`, a vote for the measure.
    Yes,
    /// `no`, a vote against the measure.
    No,
    /// `no-position`, no advice.
    NoPosition,
    /// `abstain`, leaving the measure blank.
    Abstain,
}

/// Where a contest or measure sits in the ballot tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    /// The identifier of the innermost ballot that encloses the question.
    pub ballot: String,
    /// The title of the section that encloses the question, if there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// The identifiers of every ballot that encloses the question, outermost first, ending with
    /// [`ballot`](Self::ballot).
    pub path: Vec<String>,
}

/// One contest or measure in the flat list of every question, for tables of contents and views of
/// one electorate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuestionEntry {
    /// Whether the question is a contest or a measure.
    pub kind: QuestionKind,
    /// The question's identifier, without its `@`.
    pub id: String,
    /// The question's title.
    pub title: String,
    /// Where the question sits in the ballot tree.
    pub location: Location,
    /// Whether a voter must answer the question.
    pub required: bool,
}

/// Whether a question is a contest or a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionKind {
    /// A race between campaigns for an office.
    Contest,
    /// A question answered yes or no.
    Measure,
}

/// Totals across the whole document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// Every ballot, root ballots and sub-ballots alike.
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
