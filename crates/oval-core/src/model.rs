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
