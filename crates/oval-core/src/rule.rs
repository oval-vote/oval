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
    /// A block-form campaign has fields but no member, so a recommendation has nothing to name.
    CampaignWithoutMembers,
    /// Two members of one campaign have the same identifier. Each member after the first is its
    /// own diagnostic.
    DuplicateMember,
    /// Members of two campaigns in one contest have the same identifier. Each member that repeats
    /// one of an earlier campaign is its own diagnostic.
    MemberInTwoCampaigns,
    /// A candidate in one campaign of a contest has a party that is a member of another, so the
    /// ballot offers both as separate choices.
    PartyInTwoCampaigns,
    /// The number of votes after `multiple` is less than 2.
    InvalidMultipleCount,
    /// The number after `max` is 0.
    InvalidRankedMax,
    /// The number after `seats` is 0.
    InvalidSeatCount,
    /// The number after `min` is 0, or exceeds the `max` of a `ranked` contest or the number of
    /// votes of a `multiple` contest, so a ballot could never meet it.
    InvalidMinimum,
    /// The number after `multiple`, `seats`, `max`, or `min` is greater than 9007199254740991, the
    /// largest integer a JSON consumer can be relied on to read exactly.
    IntegerTooLarge,
    /// `all` is written with `max` or `min`, though ranking every choice leaves no room for
    /// either limit.
    RankedAllWithLimit,
    /// A `retention` contest has no campaign, or more than one. Each campaign after the first is
    /// its own diagnostic.
    RetentionCampaignCount,
    /// A campaign of a `retention` contest is a party campaign, though a retention question is about
    /// one candidate.
    RetentionPartyCampaign,
    /// A contest or measure has no recommendation.
    MissingRecommendation,
    /// A contest or measure has more than one recommendation. Each one after the first is its own
    /// diagnostic.
    RepeatedRecommendation,
    /// A `support` or `oppose` reference resolves to a candidate or party that is a member of no
    /// campaign in its contest.
    CampaignNotInContest,
    /// A recommendation has more than one `oppose` line. Each one after the first is its own
    /// diagnostic.
    RepeatedOppose,
    /// A choice appears more than once across a recommendation's `support` and `oppose` lines.
    /// Each repeat is its own diagnostic.
    RepeatedStance,
    /// A `single` contest's recommendation has more than one `support` line, though a vote for
    /// one has no second choice.
    TiersInSingle,
    /// A `retention` contest's recommendation is not a single `support` or `oppose` line naming
    /// only the contest's campaign, `no-position`, or `abstain`.
    RetentionStance,
    /// A `ranked` contest's recommendation names more choices in its `support` lines than its
    /// `max`.
    SupportOverMax,
    /// A required question is recommended `abstain`, though it cannot be left blank.
    AbstainOnRequired,
    /// A recommendation has a `write-in`, though its contest is exhaustive.
    WriteInNotAccepted,
    /// `extensible` is written before a `retention` contest, which has nothing to write in.
    ExtensibleRetention,
    /// A Markdown body uses a construct outside the profile, such as code, raw HTML, an image, or a
    /// thematic break, or has a link whose destination is not an absolute `http` or `https` URL.
    /// Each construct is its own diagnostic.
    MarkdownProfile,
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
    /// A `multiple` contest's recommendation names more choices in its `support` lines than the
    /// voter has votes. A warning.
    SupportOverCount,
    /// A recommendation's `support` lines name fewer choices than a ballot must mark, so a reader
    /// who marks only those casts a ballot that does not count. A warning.
    SupportBelowMinimum,
    /// A write-in names a candidate already printed on the ballot in the same contest. A warning.
    WriteInOnBallot,
    /// A measure has `if_yes` or `if_no` but not both, so a reader learns what only one outcome
    /// does. A warning.
    UnpairedOutcome,
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
            Self::CampaignWithoutMembers => "campaign-without-members",
            Self::DuplicateMember => "duplicate-member",
            Self::MemberInTwoCampaigns => "member-in-two-campaigns",
            Self::PartyInTwoCampaigns => "party-in-two-campaigns",
            Self::InvalidMultipleCount => "invalid-multiple-count",
            Self::InvalidRankedMax => "invalid-ranked-max",
            Self::InvalidSeatCount => "invalid-seat-count",
            Self::InvalidMinimum => "invalid-minimum",
            Self::IntegerTooLarge => "integer-too-large",
            Self::RankedAllWithLimit => "ranked-all-with-limit",
            Self::RetentionCampaignCount => "retention-campaign-count",
            Self::RetentionPartyCampaign => "retention-party-campaign",
            Self::MissingRecommendation => "missing-recommendation",
            Self::RepeatedRecommendation => "repeated-recommendation",
            Self::CampaignNotInContest => "campaign-not-in-contest",
            Self::RepeatedOppose => "repeated-oppose",
            Self::RepeatedStance => "repeated-stance",
            Self::TiersInSingle => "tiers-in-single",
            Self::RetentionStance => "retention-stance",
            Self::SupportOverMax => "support-over-max",
            Self::AbstainOnRequired => "abstain-on-required",
            Self::WriteInNotAccepted => "write-in-not-accepted",
            Self::ExtensibleRetention => "extensible-retention",
            Self::MarkdownProfile => "markdown-profile",
            Self::InsecureUrl => "insecure-url",
            Self::EmptyString => "empty-string",
            Self::UnusedDeclaration => "unused-declaration",
            Self::DuplicatePartyName => "duplicate-party-name",
            Self::DuplicateBallotTitle => "duplicate-ballot-title",
            Self::SupportOverCount => "support-over-count",
            Self::SupportBelowMinimum => "support-below-minimum",
            Self::WriteInOnBallot => "write-in-on-ballot",
            Self::UnpairedOutcome => "unpaired-outcome",
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
            | Self::CampaignWithoutMembers
            | Self::DuplicateMember
            | Self::MemberInTwoCampaigns
            | Self::PartyInTwoCampaigns
            | Self::InvalidMultipleCount
            | Self::InvalidRankedMax
            | Self::InvalidSeatCount
            | Self::InvalidMinimum
            | Self::IntegerTooLarge
            | Self::RankedAllWithLimit
            | Self::RetentionCampaignCount
            | Self::RetentionPartyCampaign
            | Self::MissingRecommendation
            | Self::RepeatedRecommendation
            | Self::CampaignNotInContest
            | Self::RepeatedOppose
            | Self::RepeatedStance
            | Self::TiersInSingle
            | Self::RetentionStance
            | Self::SupportOverMax
            | Self::AbstainOnRequired
            | Self::WriteInNotAccepted
            | Self::ExtensibleRetention
            | Self::MarkdownProfile => Severity::Error,
            Self::InsecureUrl
            | Self::EmptyString
            | Self::UnusedDeclaration
            | Self::DuplicatePartyName
            | Self::DuplicateBallotTitle
            | Self::SupportOverCount
            | Self::SupportBelowMinimum
            | Self::WriteInOnBallot
            | Self::UnpairedOutcome => Severity::Warning,
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
                ValidationRule::CampaignWithoutMembers,
                "campaign-without-members",
            ),
            (ValidationRule::DuplicateMember, "duplicate-member"),
            (
                ValidationRule::MemberInTwoCampaigns,
                "member-in-two-campaigns",
            ),
            (
                ValidationRule::PartyInTwoCampaigns,
                "party-in-two-campaigns",
            ),
            (
                ValidationRule::InvalidMultipleCount,
                "invalid-multiple-count",
            ),
            (ValidationRule::InvalidRankedMax, "invalid-ranked-max"),
            (ValidationRule::InvalidSeatCount, "invalid-seat-count"),
            (ValidationRule::InvalidMinimum, "invalid-minimum"),
            (ValidationRule::IntegerTooLarge, "integer-too-large"),
            (ValidationRule::RankedAllWithLimit, "ranked-all-with-limit"),
            (
                ValidationRule::RetentionCampaignCount,
                "retention-campaign-count",
            ),
            (
                ValidationRule::RetentionPartyCampaign,
                "retention-party-campaign",
            ),
            (
                ValidationRule::MissingRecommendation,
                "missing-recommendation",
            ),
            (
                ValidationRule::RepeatedRecommendation,
                "repeated-recommendation",
            ),
            (
                ValidationRule::CampaignNotInContest,
                "campaign-not-in-contest",
            ),
            (ValidationRule::RepeatedOppose, "repeated-oppose"),
            (ValidationRule::RepeatedStance, "repeated-stance"),
            (ValidationRule::TiersInSingle, "tiers-in-single"),
            (ValidationRule::RetentionStance, "retention-stance"),
            (ValidationRule::SupportOverMax, "support-over-max"),
            (ValidationRule::AbstainOnRequired, "abstain-on-required"),
            (ValidationRule::WriteInNotAccepted, "write-in-not-accepted"),
            (ValidationRule::ExtensibleRetention, "extensible-retention"),
            (ValidationRule::MarkdownProfile, "markdown-profile"),
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
            (ValidationRule::SupportOverCount, "support-over-count"),
            (ValidationRule::SupportBelowMinimum, "support-below-minimum"),
            (ValidationRule::WriteInOnBallot, "write-in-on-ballot"),
            (ValidationRule::UnpairedOutcome, "unpaired-outcome"),
        ] {
            assert_eq!(rule.spec_id(), id);
            assert_eq!(rule.severity(), Severity::Warning);
        }
    }
}
