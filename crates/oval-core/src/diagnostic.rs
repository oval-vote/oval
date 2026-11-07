//! The code, severity, position, and message of every problem the checker reports.

use std::fmt;

use oval_syntax::TextRange;
use oval_syntax::lexer::LexicalErrorKind;
use oval_syntax::parser::ParseErrorKind;
use serde::{Serialize, Serializer};

/// The address of the edition of the specification this implementation follows.
///
/// [`DiagnosticCode::url`] appends a fragment to it to address one entry.
pub const SPECIFICATION_URL: &str = "https://spec.oval.vote/draft/";

/// The seriousness of a diagnostic, either an error or a warning.
///
/// Serializes as `"error"` or `"warning"`. Errors order before warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The document is not valid Oval. A document with any error does not compile.
    Error,
    /// The document is valid but probably not what the author meant, Section 3.4. Warnings do not
    /// prevent compilation.
    Warning,
}

/// The validation errors and warnings of the specification, Sections 3.3 and 3.4.
///
/// Each variant is one entry of those sections, in the specification's order, and gives its
/// [`identifier`](Self::identifier), [`title`](Self::title), and [`severity`](Self::severity).
/// Lexical and syntax errors are not rules; they are the other two variants of
/// [`DiagnosticCode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationRule {
    /// The document has no version marker, or its marker names a major version this
    /// implementation does not read. When it is reported, nothing else is.
    UnsupportedVersion,
    /// `guide`, `election`, or `date` is absent at the document level; reported once per absent
    /// field.
    MissingGuideField,
    /// The document has no root ballot.
    EmptyDocument,
    /// A date does not exist on the proleptic Gregorian calendar.
    InvalidDate,
    /// A reference names an identifier that no declaration in the document declares.
    UndeclaredReference,
    /// An identifier is declared more than once, at the top level or inline, whatever the kinds
    /// of the declarations.
    DuplicateDeclaration,
    /// A reference resolves to a declaration of a kind its position does not accept. A `party`
    /// line must name a party, and a member, `support`, or `oppose` must name a candidate.
    MismatchedReference,
    /// A member that is a reference, not an inline declaration, is followed by an indented block
    /// of candidate fields.
    BlockOnReference,
    /// `designation` or `party` appears where it cannot describe a candidate, either at the
    /// campaign level of a block-form campaign or on a one-line campaign whose member is a
    /// reference.
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
    /// A contest or measure has no recommendation; `recommend no-position` is how an author takes
    /// none.
    MissingRecommendation,
    /// A contest or measure has more than one recommendation.
    RepeatedRecommendation,
    /// A `support` or `oppose` reference names a candidate who is a member of no campaign in the
    /// enclosing contest.
    CampaignNotInContest,
    /// A recommendation has more than one `oppose` line.
    RepeatedOppose,
    /// A candidate is referenced more than once across a recommendation's `support` lines and its
    /// `oppose` line.
    RepeatedStance,
    /// A `single` contest's recommendation has more than one `support` line.
    TiersInSingle,
    /// A `ranked` contest declares `max` *n* and its recommendation's `support` lines name more
    /// than *n* campaigns in total.
    SupportOverMax,
    /// A `retention` contest's recommendation is anything other than one `support` line with one
    /// reference, one `oppose` line with one reference, or `recommend no-position`.
    RetentionStance,
    /// A Markdown body contains a construct outside the profile of Section 2.2, or a link whose
    /// destination is not an absolute `http` or `https` URL. Its identifier is `markdown-profile`.
    UnsupportedMarkdown,
    /// A top-level party is named by no `party` line, or a top-level candidate is a member of no
    /// campaign. This rule is a warning.
    UnusedDeclaration,
    /// Two party declarations have the same name and different identifiers. This rule is a
    /// warning.
    DuplicatePartyName,
    /// Two root ballots, or two sibling sub-ballots, have identical titles. This rule is a
    /// warning.
    DuplicateBallotTitle,
    /// A `multiple` contest elects *n* and its recommendation's `support` lines name more than
    /// *n* campaigns in total. This rule is a warning.
    SupportOverCount,
    /// A `support` or `oppose` reference names a member of a multi-member campaign other than
    /// its first member. This rule is a warning.
    NonHeadReference,
    /// A measure has exactly one of `if_yes` and `if_no`. This rule is a warning.
    UnpairedOutcome,
    /// A URL, or a link destination inside a Markdown body, uses the `http` scheme rather than
    /// `https`. This rule is a warning.
    InsecureUrl,
}

impl ValidationRule {
    /// Every rule, in the specification's order. The errors of Section 3.3 come first, then the
    /// warnings of Section 3.4.
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
        Self::UnsupportedMarkdown,
        Self::UnusedDeclaration,
        Self::DuplicatePartyName,
        Self::DuplicateBallotTitle,
        Self::SupportOverCount,
        Self::NonHeadReference,
        Self::UnpairedOutcome,
        Self::InsecureUrl,
    ];

    /// The identifier the specification gives this rule, such as `invalid-date`.
    ///
    /// Identifiers are stable across editions, so tools report diagnostics by identifier.
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
            Self::UnsupportedMarkdown => "markdown-profile",
            Self::UnusedDeclaration => "unused-declaration",
            Self::DuplicatePartyName => "duplicate-party-name",
            Self::DuplicateBallotTitle => "duplicate-ballot-title",
            Self::SupportOverCount => "support-over-count",
            Self::NonHeadReference => "non-head-reference",
            Self::UnpairedOutcome => "unpaired-outcome",
            Self::InsecureUrl => "insecure-url",
        }
    }

    /// The title of this rule's entry in the specification, such as `Invalid Date`.
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
            Self::UnsupportedMarkdown => "Unsupported Markdown",
            Self::UnusedDeclaration => "Unused Declaration",
            Self::DuplicatePartyName => "Duplicate Party Name",
            Self::DuplicateBallotTitle => "Duplicate Ballot Title",
            Self::SupportOverCount => "Support Beyond Multiple Count",
            Self::NonHeadReference => "Non-Head Reference",
            Self::UnpairedOutcome => "Unpaired Outcome",
            Self::InsecureUrl => "Insecure URL",
        }
    }

    /// Whether breaking this rule makes the document invalid.
    ///
    /// The rules of Section 3.3 are errors and those of Section 3.4 are warnings.
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
            | Self::RetentionStance
            | Self::UnsupportedMarkdown => Severity::Error,
            Self::UnusedDeclaration
            | Self::DuplicatePartyName
            | Self::DuplicateBallotTitle
            | Self::SupportOverCount
            | Self::NonHeadReference
            | Self::UnpairedOutcome
            | Self::InsecureUrl => Severity::Warning,
        }
    }
}

/// The subject of a diagnostic, which is one entry of the specification's Section 3.
///
/// The three variants are the three families of Section 3 and the three stages of checking that
/// report them. Every code has an [`identifier`](Self::identifier), a [`title`](Self::title), and
/// a [`severity`](Self::severity), taken from the kind or rule inside it, and an address on the
/// specification's site built from the title. [`anchor`](Self::anchor) is `sec-` followed by the
/// title with each space replaced by `-`, and [`url`](Self::url) is [`SPECIFICATION_URL`]
/// followed by `#` and the anchor.
///
/// Displays and serializes as its identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// A lexical error, Section 3.1, which is text the lexer could not tokenize or a flaw within
    /// a token.
    Lexical(LexicalErrorKind),
    /// A syntax error, Section 3.2, which is a token the grammar did not expect.
    Syntax(ParseErrorKind),
    /// A validation error or warning, Sections 3.3 and 3.4, which is a rule that a well-formed
    /// document breaks.
    Validation(ValidationRule),
}

impl DiagnosticCode {
    /// Every code, in the specification's order. Lexical errors come first, then syntax errors,
    /// then validation rules.
    ///
    /// This is the catalog a tool uses to list what it can report, with each code's identifier,
    /// title, severity, and address.
    ///
    /// # Examples
    ///
    /// ```
    /// use oval_core::{DiagnosticCode, Severity};
    ///
    /// let all = DiagnosticCode::all();
    /// let identifiers: Vec<&str> = all.iter().map(|code| code.identifier()).collect();
    /// assert_eq!(identifiers.first(), Some(&"illegal-tab"));
    /// assert_eq!(identifiers.last(), Some(&"insecure-url"));
    /// assert!(identifiers.contains(&"missing-guide-field"));
    ///
    /// let warnings = all.iter().filter(|code| code.severity() == Severity::Warning).count();
    /// assert_eq!(warnings, 7);
    /// ```
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
    ///
    /// Identifiers are unique across the three families and stable across editions.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Lexical(kind) => kind.identifier(),
            Self::Syntax(kind) => kind.identifier(),
            Self::Validation(rule) => rule.identifier(),
        }
    }

    /// The title of this code's entry in the specification, such as `Illegal Tab`.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Lexical(kind) => kind.title(),
            Self::Syntax(kind) => kind.title(),
            Self::Validation(rule) => rule.title(),
        }
    }

    /// The severity of a diagnostic with this code.
    ///
    /// Lexical and syntax errors are always errors; a validation rule's severity is
    /// [`ValidationRule::severity`].
    pub const fn severity(self) -> Severity {
        match self {
            Self::Lexical(_) | Self::Syntax(_) => Severity::Error,
            Self::Validation(rule) => rule.severity(),
        }
    }

    /// The fragment of this code's entry on the specification's site, which is `sec-` followed by
    /// the [`title`](Self::title) with each space replaced by `-`, such as `sec-Illegal-Tab`.
    pub fn anchor(self) -> String {
        format!("sec-{}", self.title().replace(' ', "-"))
    }

    /// The address of this code's entry on the specification's site. It is [`SPECIFICATION_URL`]
    /// followed by `#` and the [`anchor`](Self::anchor).
    ///
    /// # Examples
    ///
    /// ```
    /// use oval_core::{DiagnosticCode, ValidationRule};
    ///
    /// let code = DiagnosticCode::Validation(ValidationRule::TiersInSingle);
    /// assert_eq!(code.anchor(), "sec-Tiers-in-a-Single-Contest");
    /// assert_eq!(code.url(), "https://spec.oval.vote/draft/#sec-Tiers-in-a-Single-Contest");
    /// ```
    pub fn url(self) -> String {
        format!("{SPECIFICATION_URL}#{}", self.anchor())
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.identifier())
    }
}

/// One problem found in a document, with a code, a severity, a range, and a message.
///
/// Serializes as an object with `code` (the identifier), `severity`, `range` (`start` and `end`
/// as byte offsets), and `message`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// The entry of the specification the diagnostic reports.
    pub code: DiagnosticCode,
    /// The severity, which [`new`](Self::new) takes from the code.
    pub severity: Severity,
    /// The location, as a range of byte offsets into the UTF-8 source, empty when the specification
    /// places the diagnostic at a point.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// A one-sentence explanation for the author.
    ///
    /// Messages are this implementation's wording and may change; the code is the stable part.
    pub message: String,
}

impl Diagnostic {
    /// Creates a diagnostic for `code` at `range`, with the severity the code carries.
    ///
    /// # Examples
    ///
    /// ```
    /// use oval_core::{Diagnostic, DiagnosticCode, Severity, ValidationRule};
    /// use oval_syntax::TextRange;
    ///
    /// let code = DiagnosticCode::Validation(ValidationRule::UnusedDeclaration);
    /// let range = TextRange::new(7.into(), 30.into());
    /// let message = "`@gop` is declared but no candidate belongs to it".to_owned();
    /// let diagnostic = Diagnostic::new(code, range, message);
    /// assert_eq!(diagnostic.severity, Severity::Warning);
    /// assert_eq!(diagnostic.code.identifier(), "unused-declaration");
    /// assert_eq!(u32::from(diagnostic.range.start()), 7);
    /// ```
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
        assert_eq!(all.len(), 7 + 13 + 34);
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
                "insecure-url",
            ]
        );
    }
}
