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
