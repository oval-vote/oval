use std::fmt;

use oval_syntax::TextRange;
use serde::{Serialize, Serializer};

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The document is not valid Oval.
    Error,
    /// The document is valid but probably not what the author meant.
    Warning,
}

/// A validation rule from Section 3 of the specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Rule {
    /// V1: identifiers are declared exactly once and every reference resolves.
    Identifiers,
    /// V2: `guide`, `election`, and `date` are present.
    RequiredGuideFields,
    /// V3: a member block follows only an inline declaration.
    MemberBlocksOnlyOnDeclarations,
    /// V4: `designation` and `party` appear only where the candidate is declared.
    CandidateFieldsInPlace,
    /// V5: one `oppose`, and no candidate twice in a recommendation.
    RecommendationShape,
    /// V6: recommendation references are campaigns of the same contest.
    RecommendationReferencesInContest,
    /// V7: a candidate is in at most one campaign per contest.
    OneCampaignPerCandidate,
    /// V8: no candidate appears twice in one campaign.
    NoDuplicateMembers,
    /// V9: contest kind arguments are present and in range.
    ContestKindArguments,
    /// V10: every contest and measure has exactly one recommendation.
    EveryQuestionHasARecommendation,
    /// V11: recommendation shapes allowed by the contest kind.
    KindSpecificRecommendation,
    /// V12: ballots and sections have items.
    BallotStructure,
    /// V13: dates exist on the calendar.
    Dates,
    /// V14: declared parties and candidates are used.
    UnusedDeclarations,
    /// V15: two parties do not share a name.
    DuplicatePartyNames,
    /// V16: sibling ballots do not share a title.
    SuspiciousBallots,
    /// V17: multi-member campaigns are named by their head of ticket.
    HeadOfTicketReferences,
    /// V18: `if_yes` and `if_no` appear together.
    PairedYesNoStatements,
}

impl Rule {
    /// The identifier used in the specification and by tools, such as `V1`.
    pub fn id(self) -> &'static str {
        match self {
            Self::Identifiers => "V1",
            Self::RequiredGuideFields => "V2",
            Self::MemberBlocksOnlyOnDeclarations => "V3",
            Self::CandidateFieldsInPlace => "V4",
            Self::RecommendationShape => "V5",
            Self::RecommendationReferencesInContest => "V6",
            Self::OneCampaignPerCandidate => "V7",
            Self::NoDuplicateMembers => "V8",
            Self::ContestKindArguments => "V9",
            Self::EveryQuestionHasARecommendation => "V10",
            Self::KindSpecificRecommendation => "V11",
            Self::BallotStructure => "V12",
            Self::Dates => "V13",
            Self::UnusedDeclarations => "V14",
            Self::DuplicatePartyNames => "V15",
            Self::SuspiciousBallots => "V16",
            Self::HeadOfTicketReferences => "V17",
            Self::PairedYesNoStatements => "V18",
        }
    }
}

/// What a diagnostic is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// The lexer or parser rejected the text.
    Syntax,
    /// A validation rule failed.
    Rule(Rule),
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax => f.write_str("syntax"),
            Self::Rule(rule) => f.write_str(rule.id()),
        }
    }
}

/// One problem found in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// What the diagnostic is about.
    pub code: DiagnosticCode,
    /// How serious it is.
    pub severity: Severity,
    /// Where it is.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// A one-sentence explanation for the author.
    pub message: String,
}

impl Serialize for DiagnosticCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_range<S: Serializer>(range: &TextRange, serializer: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Range {
        start: u32,
        end: u32,
    }
    Range {
        start: range.start().into(),
        end: range.end().into(),
    }
    .serialize(serializer)
}
