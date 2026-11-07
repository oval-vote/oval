use std::fmt;

use oval_syntax::TextRange;
use oval_syntax::lexer::LexicalErrorKind;
use oval_syntax::parser::ParseErrorKind;
use serde::{Serialize, Serializer};

/// Where the specification this implementation follows is published.
pub const SPECIFICATION_URL: &str = "https://spec.oval.vote/draft/";

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The document is not valid Oval.
    Error,
    /// The document is valid but probably not what the author meant.
    Warning,
}

/// The validation errors and warnings of the specification, Sections 3.3 and 3.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationRule {
    /// The document has no version marker, or names an unsupported major version.
    UnsupportedVersion,
    /// `guide`, `election`, or `date` is absent at the document level.
    MissingGuideField,
    /// The document has no root ballot.
    EmptyDocument,
    /// A `date` does not exist on the calendar.
    InvalidDate,
    /// A reference names an identifier no declaration declares.
    UndeclaredReference,
    /// An identifier is declared more than once.
    DuplicateDeclaration,
    /// A reference resolves to a declaration of a kind its position does not accept.
    MismatchedReference,
    /// A member that is a reference is followed by a block of candidate fields.
    BlockOnReference,
    /// `designation` or `party` appears where it cannot describe a candidate.
    MisplacedCandidateField,
    /// A ballot has fields but no items.
    EmptyBallot,
    /// A section has fields but no items.
    EmptySection,
    /// A contest whose kind is not `retention` has no campaign.
    ContestWithoutCampaigns,
    /// A block-form campaign has fields but no member.
    CampaignWithoutMembers,
    /// A candidate appears twice among one campaign's members.
    DuplicateMember,
    /// A candidate is a member of more than one campaign in the same contest.
    CandidateInTwoCampaigns,
    /// The integer after `multiple` is less than 2.
    InvalidMultipleCount,
    /// The integer after `max` is 0.
    InvalidRankedMax,
    /// A `retention` contest has no campaign or more than one.
    RetentionCampaignCount,
    /// A contest or measure has no recommendation.
    MissingRecommendation,
    /// A contest or measure has more than one recommendation.
    RepeatedRecommendation,
    /// A `support` or `oppose` reference names a candidate in no campaign of the contest.
    CampaignNotInContest,
    /// A recommendation has more than one `oppose` line.
    RepeatedOppose,
    /// A candidate is referenced more than once across a recommendation's lines.
    RepeatedStance,
    /// A `single` contest's recommendation has more than one `support` line.
    TiersInSingle,
    /// A `ranked` contest's `support` lines name more campaigns than its `max`.
    SupportOverMax,
    /// A `retention` contest's recommendation is not one `support` or one `oppose` reference.
    RetentionStance,
    /// A top-level party or candidate declaration is never used.
    UnusedDeclaration,
    /// Two parties have the same name and different identifiers.
    DuplicatePartyName,
    /// Two root or sibling ballots have identical titles.
    DuplicateBallotTitle,
    /// A `multiple` contest's `support` lines name more campaigns than there are seats.
    SupportOverCount,
    /// A reference names a ticket member other than its first.
    NonHeadReference,
    /// A measure has exactly one of `if_yes` and `if_no`.
    UnpairedOutcome,
}

impl ValidationRule {
    /// Every rule, in the specification's order.
    pub const ALL: &'static [Self] = &[
        Self::UnsupportedVersion,
        Self::MissingGuideField,
        Self::EmptyDocument,
        Self::InvalidDate,
        Self::UndeclaredReference,
        Self::DuplicateDeclaration,
        Self::MismatchedReference,
        Self::BlockOnReference,
        Self::MisplacedCandidateField,
        Self::EmptyBallot,
        Self::EmptySection,
        Self::ContestWithoutCampaigns,
        Self::CampaignWithoutMembers,
        Self::DuplicateMember,
        Self::CandidateInTwoCampaigns,
        Self::InvalidMultipleCount,
        Self::InvalidRankedMax,
        Self::RetentionCampaignCount,
        Self::MissingRecommendation,
        Self::RepeatedRecommendation,
        Self::CampaignNotInContest,
        Self::RepeatedOppose,
        Self::RepeatedStance,
        Self::TiersInSingle,
        Self::SupportOverMax,
        Self::RetentionStance,
        Self::UnusedDeclaration,
        Self::DuplicatePartyName,
        Self::DuplicateBallotTitle,
        Self::SupportOverCount,
        Self::NonHeadReference,
        Self::UnpairedOutcome,
    ];

    /// The identifier the specification gives this rule, such as `invalid-date`.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "unsupported-version",
            Self::MissingGuideField => "missing-guide-field",
            Self::EmptyDocument => "empty-document",
            Self::InvalidDate => "invalid-date",
            Self::UndeclaredReference => "undeclared-reference",
            Self::DuplicateDeclaration => "duplicate-declaration",
            Self::MismatchedReference => "mismatched-reference",
            Self::BlockOnReference => "block-on-reference",
            Self::MisplacedCandidateField => "misplaced-candidate-field",
            Self::EmptyBallot => "empty-ballot",
            Self::EmptySection => "empty-section",
            Self::ContestWithoutCampaigns => "contest-without-campaigns",
            Self::CampaignWithoutMembers => "campaign-without-members",
            Self::DuplicateMember => "duplicate-member",
            Self::CandidateInTwoCampaigns => "candidate-in-two-campaigns",
            Self::InvalidMultipleCount => "invalid-multiple-count",
            Self::InvalidRankedMax => "invalid-ranked-max",
            Self::RetentionCampaignCount => "retention-campaign-count",
            Self::MissingRecommendation => "missing-recommendation",
            Self::RepeatedRecommendation => "repeated-recommendation",
            Self::CampaignNotInContest => "campaign-not-in-contest",
            Self::RepeatedOppose => "repeated-oppose",
            Self::RepeatedStance => "repeated-stance",
            Self::TiersInSingle => "tiers-in-single",
            Self::SupportOverMax => "support-over-max",
            Self::RetentionStance => "retention-stance",
            Self::UnusedDeclaration => "unused-declaration",
            Self::DuplicatePartyName => "duplicate-party-name",
            Self::DuplicateBallotTitle => "duplicate-ballot-title",
            Self::SupportOverCount => "support-over-count",
            Self::NonHeadReference => "non-head-reference",
            Self::UnpairedOutcome => "unpaired-outcome",
        }
    }

    /// The title of this rule's entry in the specification.
    pub const fn title(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "Unsupported Version",
            Self::MissingGuideField => "Missing Guide Field",
            Self::EmptyDocument => "Empty Document",
            Self::InvalidDate => "Invalid Date",
            Self::UndeclaredReference => "Undeclared Reference",
            Self::DuplicateDeclaration => "Duplicate Declaration",
            Self::MismatchedReference => "Mismatched Reference",
            Self::BlockOnReference => "Block on a Reference",
            Self::MisplacedCandidateField => "Misplaced Candidate Field",
            Self::EmptyBallot => "Empty Ballot",
            Self::EmptySection => "Empty Section",
            Self::ContestWithoutCampaigns => "Contest Without Campaigns",
            Self::CampaignWithoutMembers => "Campaign Without Members",
            Self::DuplicateMember => "Duplicate Member",
            Self::CandidateInTwoCampaigns => "Candidate in Two Campaigns",
            Self::InvalidMultipleCount => "Invalid Multiple Count",
            Self::InvalidRankedMax => "Invalid Ranked Max",
            Self::RetentionCampaignCount => "Retention Campaign Count",
            Self::MissingRecommendation => "Missing Recommendation",
            Self::RepeatedRecommendation => "Repeated Recommendation",
            Self::CampaignNotInContest => "Campaign Not in Contest",
            Self::RepeatedOppose => "Repeated Oppose",
            Self::RepeatedStance => "Repeated Stance",
            Self::TiersInSingle => "Tiers in a Single Contest",
            Self::SupportOverMax => "Support Beyond Ranked Max",
            Self::RetentionStance => "Retention Stance",
            Self::UnusedDeclaration => "Unused Declaration",
            Self::DuplicatePartyName => "Duplicate Party Name",
            Self::DuplicateBallotTitle => "Duplicate Ballot Title",
            Self::SupportOverCount => "Support Beyond Multiple Count",
            Self::NonHeadReference => "Non-Head Reference",
            Self::UnpairedOutcome => "Unpaired Outcome",
        }
    }

    /// Whether breaking this rule makes the document invalid.
    pub const fn severity(self) -> Severity {
        match self {
            Self::UnsupportedVersion
            | Self::MissingGuideField
            | Self::EmptyDocument
            | Self::InvalidDate
            | Self::UndeclaredReference
            | Self::DuplicateDeclaration
            | Self::MismatchedReference
            | Self::BlockOnReference
            | Self::MisplacedCandidateField
            | Self::EmptyBallot
            | Self::EmptySection
            | Self::ContestWithoutCampaigns
            | Self::CampaignWithoutMembers
            | Self::DuplicateMember
            | Self::CandidateInTwoCampaigns
            | Self::InvalidMultipleCount
            | Self::InvalidRankedMax
            | Self::RetentionCampaignCount
            | Self::MissingRecommendation
            | Self::RepeatedRecommendation
            | Self::CampaignNotInContest
            | Self::RepeatedOppose
            | Self::RepeatedStance
            | Self::TiersInSingle
            | Self::SupportOverMax
            | Self::RetentionStance => Severity::Error,
            Self::UnusedDeclaration
            | Self::DuplicatePartyName
            | Self::DuplicateBallotTitle
            | Self::SupportOverCount
            | Self::NonHeadReference
            | Self::UnpairedOutcome => Severity::Warning,
        }
    }
}

/// What a diagnostic is about: one entry of the specification's Section 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// A lexical error, Section 3.1.
    Lexical(LexicalErrorKind),
    /// A syntax error, Section 3.2.
    Syntax(ParseErrorKind),
    /// A validation error or warning, Sections 3.3 and 3.4.
    Validation(ValidationRule),
}

impl DiagnosticCode {
    /// Every code, in the specification's order.
    pub fn all() -> Vec<Self> {
        LexicalErrorKind::ALL
            .iter()
            .map(|kind| Self::Lexical(*kind))
            .chain(ParseErrorKind::ALL.iter().map(|kind| Self::Syntax(*kind)))
            .chain(
                ValidationRule::ALL
                    .iter()
                    .map(|rule| Self::Validation(*rule)),
            )
            .collect()
    }

    /// The identifier the specification gives this code, such as `illegal-tab`.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Lexical(kind) => kind.identifier(),
            Self::Syntax(kind) => kind.identifier(),
            Self::Validation(rule) => rule.identifier(),
        }
    }

    /// The title of this code's entry in the specification.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Lexical(kind) => kind.title(),
            Self::Syntax(kind) => kind.title(),
            Self::Validation(rule) => rule.title(),
        }
    }

    /// How serious a diagnostic with this code is.
    pub const fn severity(self) -> Severity {
        match self {
            Self::Lexical(_) | Self::Syntax(_) => Severity::Error,
            Self::Validation(rule) => rule.severity(),
        }
    }

    /// The fragment of this code's entry on the specification's site.
    pub fn anchor(self) -> String {
        format!("sec-{}", self.title().replace(' ', "-"))
    }

    /// The address of this code's entry on the specification's site.
    pub fn url(self) -> String {
        format!("{SPECIFICATION_URL}#{}", self.anchor())
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.identifier())
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

impl Diagnostic {
    /// A diagnostic whose severity is the one its code carries.
    pub fn new(code: DiagnosticCode, range: TextRange, message: String) -> Self {
        Self {
            code,
            severity: code.severity(),
            range,
            message,
        }
    }
}

impl Serialize for DiagnosticCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.identifier())
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{DiagnosticCode, Severity, ValidationRule};

    #[test]
    fn every_code_has_a_unique_kebab_case_identifier() {
        let all = DiagnosticCode::all();
        assert_eq!(all.len(), 7 + 13 + 32);
        let identifiers: HashSet<&str> = all.iter().map(|code| code.identifier()).collect();
        assert_eq!(identifiers.len(), all.len());
        for code in &all {
            let identifier = code.identifier();
            assert!(
                identifier
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'-'),
                "{identifier}"
            );
            assert!(!code.title().is_empty(), "{identifier}");
        }
    }

    #[test]
    fn anchors_follow_the_sites_heading_ids() {
        assert_eq!(
            DiagnosticCode::Validation(ValidationRule::TiersInSingle).url(),
            "https://spec.oval.vote/draft/#sec-Tiers-in-a-Single-Contest"
        );
        assert_eq!(
            DiagnosticCode::Validation(ValidationRule::NonHeadReference).anchor(),
            "sec-Non-Head-Reference"
        );
    }

    #[test]
    fn the_warnings_are_exactly_the_specifications() {
        let warnings: Vec<&str> = ValidationRule::ALL
            .iter()
            .filter(|rule| rule.severity() == Severity::Warning)
            .map(|rule| rule.identifier())
            .collect();
        assert_eq!(
            warnings,
            [
                "unused-declaration",
                "duplicate-party-name",
                "duplicate-ballot-title",
                "support-over-count",
                "non-head-reference",
                "unpaired-outcome",
            ]
        );
    }
}
