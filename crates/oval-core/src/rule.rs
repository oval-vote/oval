use crate::Severity;

/// A rule a well-formed document can break, one for each validation error and warning in the
/// specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationRule {
    /// The document has no version marker, or its marker names no major version or one this
    /// implementation does not support.
    ///
    /// A document that breaks this rule is reported with this diagnostic alone, since the rest of
    /// it may not mean what this version of the language says.
    UnsupportedVersion,
    /// `guide`, `election`, or `date` is absent at the document level. Each absent field is its own
    /// diagnostic.
    MissingGuideField,
    /// The document has no root ballot.
    EmptyDocument,
    /// A date does not exist on the calendar, such as `2026-02-30`.
    InvalidDate,
    /// A reference names an identifier that nothing in the document declares.
    UndeclaredReference,
    /// An identifier is declared more than once, whatever the kinds of the declarations. Each
    /// declaration after the first is its own diagnostic, and references resolve to the first.
    DuplicateDeclaration,
    /// A reference resolves to a declaration of a kind its position does not accept, such as a
    /// candidate's `party` field naming another candidate.
    MismatchedReference,
    /// A reference, not an inline declaration, has a block of fields below it, though a reference
    /// has nothing to declare.
    BlockOnReference,
    /// `designation` or `party` is written at a campaign's level where it describes no candidate,
    /// in a block-form campaign, or in a one-line campaign whose member is a reference.
    MisplacedCandidateField,
    /// A ballot has fields but no items.
    EmptyBallot,
    /// A section has fields but no items.
    EmptySection,
    /// A `single`, `multiple`, or `ranked` contest has no campaign.
    ContestWithoutCampaigns,
    /// The number of votes after `multiple` is less than 2.
    InvalidMultipleCount,
    /// The number after `max` is 0.
    InvalidRankedMax,
    /// The number after `seats` is 0.
    InvalidSeatCount,
    /// The number after `min` is 0, or exceeds the `max` of a `ranked` contest or the number of
    /// votes of a `multiple` contest, so a ballot could never meet it.
    InvalidMinimum,
    /// `all` is written with `max` or `min`, though ranking every choice leaves no room for
    /// either limit.
    RankedAllWithLimit,
    /// A `retention` contest has no campaign, or more than one. Each campaign after the first is
    /// its own diagnostic.
    RetentionCampaignCount,
    /// A campaign of a `retention` contest is a party campaign, though a retention question is about
    /// one candidate.
    RetentionPartyCampaign,
    /// A URL uses `http` rather than `https`, so a reader following it is not protected from
    /// tampering. A warning.
    InsecureUrl,
    /// A string has no characters, or only spaces and tabs, so it shows the reader a blank. A
    /// warning.
    EmptyString,
    /// A top-level party or candidate is never used, because no `party` field and no campaign
    /// member resolves to it. A warning.
    ///
    /// A duplicate declaration is never used, since references resolve to the first declaration
    /// of an identifier. A party or candidate declared inline is used where it is declared.
    UnusedDeclaration,
    /// Two parties, declared at the top level or inline, have the same name and different
    /// identifiers, so a reader cannot tell them apart. A warning.
    DuplicatePartyName,
    /// Two root ballots, or two sub-ballots of the same ballot, have the same title, so a reader
    /// cannot tell them apart. A warning.
    DuplicateBallotTitle,
}

impl ValidationRule {
    /// Returns this rule's identifier in the specification, such as `"missing-guide-field"`.
    pub fn spec_id(self) -> &'static str {
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
            Self::InvalidMultipleCount => "invalid-multiple-count",
            Self::InvalidRankedMax => "invalid-ranked-max",
            Self::InvalidSeatCount => "invalid-seat-count",
            Self::InvalidMinimum => "invalid-minimum",
            Self::RankedAllWithLimit => "ranked-all-with-limit",
            Self::RetentionCampaignCount => "retention-campaign-count",
            Self::RetentionPartyCampaign => "retention-party-campaign",
            Self::InsecureUrl => "insecure-url",
            Self::EmptyString => "empty-string",
            Self::UnusedDeclaration => "unused-declaration",
            Self::DuplicatePartyName => "duplicate-party-name",
            Self::DuplicateBallotTitle => "duplicate-ballot-title",
        }
    }

    /// Returns how serious breaking this rule is.
    pub fn severity(self) -> Severity {
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
            | Self::InvalidMultipleCount
            | Self::InvalidRankedMax
            | Self::InvalidSeatCount
            | Self::InvalidMinimum
            | Self::RankedAllWithLimit
            | Self::RetentionCampaignCount
            | Self::RetentionPartyCampaign => Severity::Error,
            Self::InsecureUrl
            | Self::EmptyString
            | Self::UnusedDeclaration
            | Self::DuplicatePartyName
            | Self::DuplicateBallotTitle => Severity::Warning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValidationRule;
    use crate::Severity;

    #[test]
    fn each_error_has_its_identifier() {
        for (rule, id) in [
            (ValidationRule::UnsupportedVersion, "unsupported-version"),
            (ValidationRule::MissingGuideField, "missing-guide-field"),
            (ValidationRule::EmptyDocument, "empty-document"),
            (ValidationRule::InvalidDate, "invalid-date"),
            (ValidationRule::UndeclaredReference, "undeclared-reference"),
            (
                ValidationRule::DuplicateDeclaration,
                "duplicate-declaration",
            ),
            (ValidationRule::MismatchedReference, "mismatched-reference"),
            (ValidationRule::BlockOnReference, "block-on-reference"),
            (
                ValidationRule::MisplacedCandidateField,
                "misplaced-candidate-field",
            ),
            (ValidationRule::EmptyBallot, "empty-ballot"),
            (ValidationRule::EmptySection, "empty-section"),
            (
                ValidationRule::ContestWithoutCampaigns,
                "contest-without-campaigns",
            ),
            (
                ValidationRule::InvalidMultipleCount,
                "invalid-multiple-count",
            ),
            (ValidationRule::InvalidRankedMax, "invalid-ranked-max"),
            (ValidationRule::InvalidSeatCount, "invalid-seat-count"),
            (ValidationRule::InvalidMinimum, "invalid-minimum"),
            (ValidationRule::RankedAllWithLimit, "ranked-all-with-limit"),
            (
                ValidationRule::RetentionCampaignCount,
                "retention-campaign-count",
            ),
            (
                ValidationRule::RetentionPartyCampaign,
                "retention-party-campaign",
            ),
        ] {
            assert_eq!(rule.spec_id(), id);
            assert_eq!(rule.severity(), Severity::Error);
        }
    }

    #[test]
    fn each_warning_has_its_identifier() {
        for (rule, id) in [
            (ValidationRule::InsecureUrl, "insecure-url"),
            (ValidationRule::EmptyString, "empty-string"),
            (ValidationRule::UnusedDeclaration, "unused-declaration"),
            (ValidationRule::DuplicatePartyName, "duplicate-party-name"),
            (
                ValidationRule::DuplicateBallotTitle,
                "duplicate-ballot-title",
            ),
        ] {
            assert_eq!(rule.spec_id(), id);
            assert_eq!(rule.severity(), Severity::Warning);
        }
    }
}
