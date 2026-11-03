//! Resolution and validation for the Oval voting guide format.

mod diagnostic;
mod index;
mod rules;
mod syntax;

pub use diagnostic::{Diagnostic, DiagnosticCode, Rule, Severity};

use oval_syntax::ast::{AstNode, Document};
use oval_syntax::parser::{ParseResult, parse};

/// Everything a tool needs to report about one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    parse: ParseResult,
    diagnostics: Vec<Diagnostic>,
}

impl Report {
    /// The syntax tree and its errors.
    pub fn parse(&self) -> &ParseResult {
        &self.parse
    }

    /// Every diagnostic, in source order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether any diagnostic is an error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

/// Parses and validates `source`.
pub fn check(source: &str) -> Report {
    let parse = parse(source);
    let mut diagnostics: Vec<Diagnostic> = parse
        .errors()
        .iter()
        .map(|error| syntax::diagnostic(source, error))
        .collect();
    if let Some(document) = Document::cast(parse.syntax()) {
        let index = index::Index::build(&document);
        diagnostics.extend(rules::check(&document, &index));
    }
    diagnostics.sort_by_key(|d| d.range.start());
    Report { parse, diagnostics }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticCode, Rule, Severity, check};

    const FIXTURE: &str = include_str!("../../oval-syntax/tests/fixtures/franklin.oval");

    #[test]
    fn a_clean_document_has_no_diagnostics() {
        let report = check(FIXTURE);
        assert_eq!(report.diagnostics(), []);
        assert!(!report.has_errors());
    }

    #[test]
    fn syntax_errors_become_error_diagnostics_with_messages() {
        let report = check("guide yes\n\tbad\n");
        assert!(report.has_errors());
        let messages: Vec<(&DiagnosticCode, Severity, &str)> = report
            .diagnostics()
            .iter()
            .filter(|d| d.code == DiagnosticCode::Syntax)
            .map(|d| (&d.code, d.severity, d.message.as_str()))
            .collect();
        assert_eq!(
            messages,
            [
                (
                    &DiagnosticCode::Syntax,
                    Severity::Error,
                    "expected a string, found `yes`"
                ),
                (
                    &DiagnosticCode::Syntax,
                    Severity::Error,
                    "tabs are not allowed outside strings and Markdown bodies"
                ),
                (
                    &DiagnosticCode::Syntax,
                    Severity::Error,
                    "expected `election`, `date`, `note`, `party`, `candidate`, or `ballot`, found unrecognized text"
                ),
                (
                    &DiagnosticCode::Syntax,
                    Severity::Error,
                    "`bad` is not a keyword"
                ),
            ]
        );
    }

    #[test]
    fn end_of_input_is_named() {
        let syntax_message = |source: &str| {
            check(source)
                .diagnostics()
                .iter()
                .find(|d| d.code == DiagnosticCode::Syntax)
                .map(|d| d.message.clone())
        };
        assert_eq!(
            syntax_message("guide").as_deref(),
            Some("expected a string, found the end of the line")
        );
        assert_eq!(
            syntax_message("ballot @b \"B\"").as_deref(),
            Some("expected an indented block, found the end of the file")
        );
    }

    fn rule_messages(source: &str, rule: Rule) -> Vec<String> {
        check(source)
            .diagnostics()
            .iter()
            .filter(|d| d.code == DiagnosticCode::Rule(rule))
            .map(|d| d.message.clone())
            .collect()
    }

    #[test]
    fn v1_duplicate_declarations_across_every_kind() {
        let source = "\
party @dem \"Democratic\"
party @dem \"Democrats\"
candidate @dem \"Dee Em\"
ballot @b \"B\"
  contest @x \"X\" single
    campaign @c \"C\"
    recommend
      support @c
  measure @x \"Also X\"
    recommend yes
";
        assert_eq!(
            rule_messages(source, Rule::Identifiers),
            [
                "`@dem` is already declared as a party",
                "`@dem` is already declared as a party",
                "`@x` is already declared as a contest",
            ]
        );
        let report = check(source);
        let second_dem = source.match_indices("@dem").nth(1).map(|(i, _)| i).unwrap();
        let range = report
            .diagnostics()
            .iter()
            .find(|d| d.code == DiagnosticCode::Rule(Rule::Identifiers))
            .unwrap()
            .range;
        assert_eq!(usize::from(range.start()), second_dem);
        assert_eq!(usize::from(range.len()), "@dem".len());
    }

    #[test]
    fn v1_undeclared_references() {
        let source = "\
candidate @a \"A\"
  party @nope
ballot @b \"B\"
  contest @x \"X\" single
    campaign @ghost
    campaign @c \"C\"
      party @missing
    recommend
      support @c @who
      oppose @ghost
";
        assert_eq!(
            rule_messages(source, Rule::Identifiers),
            [
                "`@nope` is not declared",
                "`@ghost` is not declared",
                "`@missing` is not declared",
                "`@who` is not declared",
                "`@ghost` is not declared",
            ]
        );
    }

    #[test]
    fn v1_references_must_match_the_declared_kind() {
        let source = "\
party @dem \"Democratic\"
candidate @c \"C\"
  party @c
ballot @b \"B\"
  contest @x \"X\" single
    campaign @dem
    campaign @c
    recommend
      support @dem
";
        assert_eq!(
            rule_messages(source, Rule::Identifiers),
            [
                "`@c` is a candidate, not a party",
                "`@dem` is a party, not a candidate",
                "`@dem` is a party, not a candidate",
            ]
        );
    }

    #[test]
    fn v1_inline_declarations_count_and_may_be_referenced_elsewhere() {
        let source = "\
ballot @b \"B\"
  contest @x \"X\" single
    campaign @c \"C\"
      party @green \"Green\"
    recommend
      support @c
  contest @y \"Y\" single
    campaign @c
      party @green
    recommend
      support @c
";
        assert_eq!(rule_messages(source, Rule::Identifiers), [] as [String; 0]);
    }

    #[test]
    fn v1_is_silent_on_the_fixture() {
        assert_eq!(rule_messages(FIXTURE, Rule::Identifiers), [] as [String; 0]);
    }

    #[test]
    fn v2_each_required_guide_field_is_named_when_missing() {
        assert_eq!(
            rule_messages("note:\n  x\n", Rule::RequiredGuideFields),
            [
                "the document has no `guide` field",
                "the document has no `election` field",
                "the document has no `date` field",
            ]
        );
        let source = "guide \"G\"\nelection \"E\"\ndate 2026-11-03\n";
        assert_eq!(
            rule_messages(source, Rule::RequiredGuideFields),
            [] as [String; 0]
        );
        let report = check("election \"E\"\ndate 2026-11-03\n");
        assert_eq!(u32::from(report.diagnostics()[0].range.start()), 0);
    }

    #[test]
    fn v3_a_reference_member_may_not_have_a_block() {
        let source = "\
candidate @a \"A\"
ballot @b \"B\"
  contest @x \"X\" single
    campaign
      @a
        designation \"d\"
      @c \"C\"
        designation \"ok\"
    recommend
      support @a
";
        assert_eq!(
            rule_messages(source, Rule::MemberBlocksOnlyOnDeclarations),
            ["`@a` refers to a candidate declared elsewhere, so it cannot have a block"]
        );
    }

    #[test]
    fn v4_candidate_fields_only_beside_an_inline_declaration() {
        let source = "\
candidate @a \"A\"
party @dem \"D\"
ballot @b \"B\"
  contest @x \"X\" single
    campaign @a
      designation \"d\"
      party @dem
    campaign
      @c \"C\"
      designation \"d\"
    campaign @e \"E\"
      designation \"fine\"
      party @dem
    recommend
      support @a
";
        assert_eq!(
            rule_messages(source, Rule::CandidateFieldsInPlace),
            [
                "`designation` is not allowed here: the candidate is declared elsewhere, so its fields belong on that declaration",
                "`party` is not allowed here: the candidate is declared elsewhere, so its fields belong on that declaration",
                "`designation` is not allowed here: a campaign with its members in a block puts candidate fields under each member",
            ]
        );
    }

    #[test]
    fn v5_one_oppose_and_each_campaign_named_once() {
        let source = "\
ballot @b \"B\"
  contest @x \"X\" single
    campaign
      @p \"P\" as \"President\"
      @v \"V\" as \"Vice President\"
    campaign @c \"C\"
    campaign @d \"D\"
    recommend
      support @p @v
      support @c @c
      oppose @d
      oppose @nobody
";
        assert_eq!(
            rule_messages(source, Rule::RecommendationShape),
            [
                "`@v` is on the same ticket as `@p`, which is already named",
                "`@c` is named more than once",
                "a recommendation may have only one `oppose` line",
            ]
        );
    }

    #[test]
    fn v2_through_v5_are_silent_on_the_fixture() {
        for rule in [
            Rule::RequiredGuideFields,
            Rule::MemberBlocksOnlyOnDeclarations,
            Rule::CandidateFieldsInPlace,
            Rule::RecommendationShape,
        ] {
            assert_eq!(rule_messages(FIXTURE, rule), [] as [String; 0], "{rule:?}");
        }
    }

    #[test]
    fn v6_named_candidates_must_be_in_the_contest() {
        let source = "\
candidate @elsewhere \"E\"
party @dem \"D\"
ballot @b \"B\"
  contest @x \"X\" single
    campaign @c \"C\"
    recommend
      support @c @elsewhere
      oppose @nobody @dem
";
        assert_eq!(
            rule_messages(source, Rule::RecommendationReferencesInContest),
            ["`@elsewhere` is not in any campaign of this contest"]
        );
    }

    #[test]
    fn v7_and_v8_membership_duplicates() {
        let source = "\
candidate @a \"A\"
ballot @b \"B\"
  contest @x \"X\" single
    campaign
      @a
      @a
    campaign @a
    campaign
      @p \"P\"
      @p
    recommend
      support @a
";
        assert_eq!(
            rule_messages(source, Rule::NoDuplicateMembers),
            [
                "`@a` is already a member of this campaign",
                "`@p` is already a member of this campaign"
            ]
        );
        assert_eq!(
            rule_messages(source, Rule::OneCampaignPerCandidate),
            ["`@a` is already in another campaign of this contest"]
        );
    }

    #[test]
    fn v9_kind_arguments() {
        let source = "\
ballot @b \"B\"
  contest @m \"M\" multiple 1
    campaign @a \"A\"
    recommend
      support @a
  contest @r \"R\" ranked max 0
    campaign @c \"C\"
    recommend
      support @c
  contest @j \"J\" retention
    campaign @d \"D\"
    campaign @e \"E\"
    recommend
      support @d
  contest @k \"K\" retention
    recommend no-position
  contest @ok \"OK\" multiple 2
    campaign @f \"F\"
    recommend
      support @f
";
        assert_eq!(
            rule_messages(source, Rule::ContestKindArguments),
            [
                "`multiple` needs at least 2 seats; a one-seat contest is `single`",
                "`max` must be at least 1",
                "a retention contest has exactly one campaign, not 2",
                "a retention contest has exactly one campaign, not 0",
            ]
        );
        let report = check(source);
        let first = report
            .diagnostics()
            .iter()
            .find(|d| d.code == DiagnosticCode::Rule(Rule::ContestKindArguments))
            .unwrap();
        let expected = source.find("multiple 1").unwrap();
        assert_eq!(usize::from(first.range.start()), expected);
    }

    #[test]
    fn v6_through_v9_are_silent_on_the_fixture() {
        for rule in [
            Rule::RecommendationReferencesInContest,
            Rule::OneCampaignPerCandidate,
            Rule::NoDuplicateMembers,
            Rule::ContestKindArguments,
        ] {
            assert_eq!(rule_messages(FIXTURE, rule), [] as [String; 0], "{rule:?}");
        }
    }

    #[test]
    fn v10_every_question_has_exactly_one_recommendation() {
        let source = "\
ballot @b \"B\"
  contest @none \"None\" single
    campaign @a \"A\"
  contest @two \"Two\" single
    campaign @c \"C\"
    recommend
      support @c
    recommend no-position
  measure @m \"M\"
    subtitle \"S\"
  measure @n \"N\"
    recommend yes
    recommend no
";
        assert_eq!(
            rule_messages(source, Rule::EveryQuestionHasARecommendation),
            [
                "this contest has no recommendation; write `recommend no-position` to take none",
                "this contest already has a recommendation",
                "this measure has no recommendation; write `recommend no-position` to take none",
                "this measure already has a recommendation",
            ]
        );
    }

    #[test]
    fn v11_shapes_allowed_by_each_kind() {
        let source = "\
ballot @b \"B\"
  contest @s \"S\" single
    campaign @a \"A\"
    campaign @c \"C\"
    recommend
      support @a
      support @c
  contest @m \"M\" multiple 2
    campaign @d \"D\"
    campaign @e \"E\"
    campaign @f \"F\"
    recommend
      support @d @e
      support @f
  contest @r \"R\" ranked max 2
    campaign @g \"G\"
    campaign @h \"H\"
    campaign @i \"I\"
    recommend
      support @g
      support @h
      support @i
  contest @j \"J\" retention
    campaign @k \"K\"
    recommend
      support @k
      oppose @k
";
        let report = check(source);
        let v11: Vec<(Severity, &str)> = report
            .diagnostics()
            .iter()
            .filter(|d| d.code == DiagnosticCode::Rule(Rule::KindSpecificRecommendation))
            .map(|d| (d.severity, d.message.as_str()))
            .collect();
        assert_eq!(
            v11,
            [
                (
                    Severity::Error,
                    "a vote-for-one contest takes one `support` line; put interchangeable choices on it together"
                ),
                (
                    Severity::Warning,
                    "this recommendation supports 3 candidates for 2 seats; later tiers are fallbacks"
                ),
                (
                    Severity::Error,
                    "this recommendation ranks 3 candidates but voters may rank only 2"
                ),
                (
                    Severity::Error,
                    "a retention recommendation is one `support` or one `oppose` of the single campaign"
                ),
            ]
        );
    }

    #[test]
    fn v11_accepts_the_legal_shapes() {
        let source = "\
ballot @b \"B\"
  contest @s \"S\" single
    campaign @a \"A\"
    campaign @c \"C\"
    recommend
      support @a @c
  contest @r \"R\" ranked max 3
    campaign @g \"G\"
    campaign @h \"H\"
    campaign @i \"I\"
    recommend
      support @g
      support @h @i
  contest @j \"J\" retention
    campaign @k \"K\"
    recommend
      oppose @k
";
        assert_eq!(
            rule_messages(source, Rule::KindSpecificRecommendation),
            [] as [String; 0]
        );
    }

    #[test]
    fn v12_empty_containers() {
        let source = "\
ballot @empty \"Empty\"
  note:
    Nothing here.
ballot @b \"B\"
  section \"Only a note\"
    note:
      x
  contest @c \"C\" single
    recommend no-position
  contest @j \"J\" retention
    recommend no-position
";
        assert_eq!(
            rule_messages(source, Rule::BallotStructure),
            [
                "this ballot has no contests, measures, sections, or sub-ballots",
                "this section has no contests or measures",
                "this contest has no campaigns",
            ]
        );
    }

    #[test]
    fn v13_calendar_dates() {
        for (date, valid) in [
            ("2026-11-03", true),
            ("2024-02-29", true),
            ("2023-02-29", false),
            ("2000-02-29", true),
            ("1900-02-29", false),
            ("2026-13-01", false),
            ("2026-04-31", false),
            ("2026-00-10", false),
            ("2026-06-00", false),
        ] {
            let messages = rule_messages(&format!("date {date}\n"), Rule::Dates);
            if valid {
                assert_eq!(messages, [] as [String; 0], "{date}");
            } else {
                assert_eq!(messages, [format!("{date} is not a calendar date")]);
            }
        }
    }

    #[test]
    fn v10_through_v13_are_silent_on_the_fixture() {
        for rule in [
            Rule::EveryQuestionHasARecommendation,
            Rule::KindSpecificRecommendation,
            Rule::BallotStructure,
            Rule::Dates,
        ] {
            assert_eq!(rule_messages(FIXTURE, rule), [] as [String; 0], "{rule:?}");
        }
    }
}
