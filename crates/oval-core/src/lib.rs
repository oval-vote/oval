//! Resolution and validation for the Oval voting guide format.

pub mod compile;
mod diagnostic;
mod index;
mod rules;
mod syntax;

pub use compile::Compiled;
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

/// Parses, validates, and compiles `source`. Fails with the report when the document has errors.
pub fn compile(source: &str) -> Result<Compiled, Report> {
    let report = check(source);
    if report.has_errors() {
        return Err(report);
    }
    let compiled = Document::cast(report.parse.syntax()).and_then(|d| compile::compile(&d));
    compiled.ok_or(report)
}

/// Parses and validates `source`.
pub fn check(source: &str) -> Report {
    let parse = parse(source);
    let document = Document::cast(parse.syntax());
    let mut diagnostics = Vec::new();
    if let Some(document) = &document
        && !rules::version::check(document, &mut diagnostics)
    {
        return Report { parse, diagnostics };
    }
    diagnostics.extend(
        parse
            .errors()
            .iter()
            .map(|error| syntax::diagnostic(source, error)),
    );
    if let Some(document) = &document {
        let index = index::Index::build(document);
        diagnostics.extend(rules::check(document, &index));
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

    fn span(source: &str, needle: &str) -> oval_syntax::TextRange {
        let start = u32::try_from(source.find(needle).expect("needle in source")).unwrap();
        let len = u32::try_from(needle.len()).unwrap();
        oval_syntax::TextRange::new(start.into(), (start + len).into())
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

    fn warnings(source: &str, rule: Rule) -> Vec<String> {
        check(source)
            .diagnostics()
            .iter()
            .filter(|d| d.code == DiagnosticCode::Rule(rule))
            .inspect(|d| assert_eq!(d.severity, Severity::Warning, "{}", d.message))
            .map(|d| d.message.clone())
            .collect()
    }

    #[test]
    fn v14_unused_parties_and_candidates() {
        let source = "\
party @dem \"Democratic\"
party @gop \"Republican\"
candidate @used \"U\"
  party @dem
candidate @idle \"I\"
ballot @b \"B\"
  contest @x \"X\" single
    campaign @used
    campaign @inline \"In\"
      party @green \"Green\"
    recommend
      support @used
";
        assert_eq!(
            warnings(source, Rule::UnusedDeclarations),
            [
                "`@gop` is declared but no candidate belongs to it",
                "`@idle` is declared but is not in any campaign",
            ]
        );
    }

    #[test]
    fn v15_duplicate_party_names_across_top_level_and_inline() {
        let source = "\
party @dem \"Democratic\"
party @dems \"Democratic\"
candidate @a \"A\"
  party @dem
candidate @c \"C\"
  party @d2 \"Democratic\"
ballot @b \"B\"
  contest @x \"X\" single
    campaign @a
    campaign @c
    campaign @e \"E\"
      party @dems
    recommend
      support @a
";
        assert_eq!(
            warnings(source, Rule::DuplicatePartyNames),
            [
                "`@dems` has the same name as `@dem`, \"Democratic\"",
                "`@d2` has the same name as `@dem`, \"Democratic\"",
            ]
        );
    }

    #[test]
    fn v16_sibling_ballots_with_one_title() {
        let source = "\
ballot @a \"City\"
  ballot @a1 \"District 5\"
    measure @m \"M\"
      recommend yes
  ballot @a2 \"District 5\"
    measure @n \"N\"
      recommend yes
ballot @b \"City\"
  ballot @b1 \"District 5\"
    measure @o \"O\"
      recommend yes
";
        assert_eq!(
            warnings(source, Rule::SuspiciousBallots),
            [
                "another ballot at this level is also titled \"District 5\"",
                "another ballot at this level is also titled \"City\"",
            ]
        );
    }

    #[test]
    fn v17_tickets_are_named_by_their_head() {
        let source = "\
ballot @b \"B\"
  contest @x \"X\" single
    campaign
      @p \"P\" as \"President\"
      @v \"V\" as \"Vice President\"
    campaign
      @q \"Q\" as \"President\"
      @w \"W\" as \"Vice President\"
    recommend
      support @v
      oppose @q
";
        assert_eq!(
            warnings(source, Rule::HeadOfTicketReferences),
            ["`@v` is not the head of its ticket; name `@p` instead"]
        );
    }

    #[test]
    fn v18_if_yes_and_if_no_come_together() {
        let source = "\
ballot @b \"B\"
  measure @m \"M\"
    if_yes:
      Yes.
    recommend yes
  measure @n \"N\"
    if_no:
      No.
    recommend no
  measure @o \"O\"
    if_yes:
      Yes.
    if_no:
      No.
    recommend yes
";
        assert_eq!(
            warnings(source, Rule::PairedYesNoStatements),
            [
                "`if_yes` is present without `if_no`; the ballot prints both or neither",
                "`if_no` is present without `if_yes`; the ballot prints both or neither",
            ]
        );
    }

    #[test]
    fn v14_through_v18_are_silent_on_the_fixture() {
        for rule in [
            Rule::UnusedDeclarations,
            Rule::DuplicatePartyNames,
            Rule::SuspiciousBallots,
            Rule::HeadOfTicketReferences,
            Rule::PairedYesNoStatements,
        ] {
            assert_eq!(warnings(FIXTURE, rule), [] as [String; 0], "{rule:?}");
        }
        assert_eq!(check(FIXTURE).diagnostics(), []);
    }

    #[test]
    fn the_fixture_compiles() {
        let compiled = super::compile(FIXTURE).expect("fixture compiles");
        assert_eq!(compiled.oval, "0.1");
        assert_eq!(compiled.guide.name, "Franklin Voter Guide");
        assert_eq!(compiled.guide.date, "2026-11-03");
        assert_eq!(compiled.ballots.len(), 6);
        assert_eq!(compiled.counts.contests, 44);
        assert_eq!(compiled.counts.measures, 28);
        assert_eq!(compiled.counts.parties, 6);
        assert_eq!(compiled.questions.len(), 72);
        assert_eq!(compiled.counts.candidates, compiled.candidates.len());
        assert!(compiled.candidates.iter().all(|c| !c.contests.is_empty()));

        let president = compiled
            .questions
            .iter()
            .find(|q| q.id == "president")
            .expect("president in the index");
        assert_eq!(president.location.path, ["federal"]);
        assert_eq!(
            president.location.section.as_deref(),
            Some("Executive Offices")
        );
    }

    #[test]
    fn compiled_tickets_recommendations_and_stances() {
        let source = "\
oval 0.1
guide \"G\"
election \"E\"
date 2026-11-03
party @dem \"Democratic\"
  abbreviation \"D\"
ballot @state \"State\"
  contest @gov \"Governor\" single
    campaign incumbent
      @jlee \"Jordan Lee\" as \"Governor\"
        party @dem
      @mpatel \"Maya Patel\" as \"Lieutenant Governor\"
        party @dem
    campaign @rkim \"Riley Kim\"
      party @gop \"Republican\"
    recommend
      support @jlee
      justification:
        Why.
  ballot @city \"City\"
    contest @mayor \"Mayor\" ranked max 5
      campaign incumbent @b \"Bea\"
      campaign @a \"Ada\"
      campaign withdrawn @c \"Cas\"
      recommend
        support @a
        support @b
        oppose @c
    measure @m \"M\"
      supporters none
      recommend no-position
";
        let compiled = super::compile(source).expect("compiles");
        let super::compile::BallotEntry::Contest(gov) = &compiled.ballots[0].items[0] else {
            panic!("governor first");
        };
        assert_eq!(gov.campaigns[0].label, "Jordan Lee / Maya Patel");
        assert_eq!(gov.campaigns[0].head, "jlee");
        assert_eq!(gov.campaigns[0].party.as_deref(), Some("dem"));
        assert_eq!(
            gov.campaigns[0].members[1].role.as_deref(),
            Some("Lieutenant Governor")
        );
        assert_eq!(
            gov.campaigns[0].stance,
            super::compile::Stance::Support { tier: 0 }
        );
        assert_eq!(gov.campaigns[1].stance, super::compile::Stance::None);
        assert_eq!(gov.recommendation.support_campaigns, [[0]]);
        assert_eq!(gov.location.path, ["state"]);

        let super::compile::BallotEntry::Ballot(city) = &compiled.ballots[0].items[1] else {
            panic!("city second");
        };
        assert_eq!(city.path, ["state"]);
        let super::compile::BallotEntry::Contest(mayor) = &city.items[0] else {
            panic!("mayor first");
        };
        assert_eq!(
            mayor.method,
            super::compile::Method::Ranked { max: Some(5) }
        );
        assert_eq!(mayor.location.path, ["state", "city"]);
        assert_eq!(
            mayor.campaigns[0].stance,
            super::compile::Stance::Support { tier: 1 }
        );
        assert_eq!(
            mayor.campaigns[1].stance,
            super::compile::Stance::Support { tier: 0 }
        );
        assert_eq!(mayor.campaigns[2].stance, super::compile::Stance::Oppose);
        assert!(mayor.campaigns[2].withdrawn);
        assert_eq!(mayor.recommendation.oppose_campaigns, [2]);
        let super::compile::BallotEntry::Measure(m) = &city.items[1] else {
            panic!("measure second");
        };
        assert_eq!(m.supporters, Some(vec![]));
        assert_eq!(m.opponents, None);
        assert_eq!(
            m.recommendation.position,
            super::compile::Position::NoPosition
        );

        let dem = compiled.parties.iter().find(|p| p.id == "dem").unwrap();
        assert_eq!(dem.candidates, ["jlee", "mpatel"]);
        assert_eq!(
            compiled
                .parties
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            ["dem", "gop"]
        );
        let lee = compiled.candidates.iter().find(|c| c.id == "jlee").unwrap();
        assert_eq!(lee.contests, ["gov"]);
        assert_eq!(compiled.counts.campaigns, 5);
        assert_eq!(compiled.counts.sections, 0);
    }

    #[test]
    fn optional_fields_inside_unions_are_omitted() {
        let source = "\
oval 0.1
guide \"G\"
election \"E\"
date 2026-11-03
ballot @b \"B\"
  contest @r \"R\" ranked
    campaign @a \"A\"
    recommend
      support @a
";
        let compiled = super::compile(source).expect("compiles");
        let json: serde_json::Value = serde_json::from_str(&compiled.to_json().unwrap()).unwrap();
        assert_eq!(
            json["ballots"][0]["items"][0]["method"],
            serde_json::json!({ "kind": "ranked" })
        );
    }

    #[test]
    fn v19_a_missing_marker_is_reported_alongside_everything_else() {
        let report = check("guide \"G\"\n");
        let codes: Vec<String> = report
            .diagnostics()
            .iter()
            .map(|d| d.code.to_string())
            .collect();
        assert_eq!(codes, ["V19", "V2", "V2"]);
        assert_eq!(
            rule_messages("guide \"G\"\n", Rule::SupportedVersion),
            ["the document has no version marker; its first line should be `oval 0.1`"]
        );
    }

    #[test]
    fn v19_an_unsupported_version_is_the_only_diagnostic() {
        let source = "oval 0.2\nguide yes\n";
        let report = check(source);
        assert_eq!(report.diagnostics().len(), 1);
        let diagnostic = &report.diagnostics()[0];
        assert_eq!(
            diagnostic.code,
            DiagnosticCode::Rule(Rule::SupportedVersion)
        );
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.range, span(source, "0.2"));
        assert_eq!(
            diagnostic.message,
            "this document is written to Oval 0.2, but this implementation reads Oval 0.1"
        );
        assert_eq!(check("oval 1.0\n").diagnostics().len(), 1);
    }

    #[test]
    fn v19_leaves_a_malformed_marker_to_the_parser() {
        let report = check("oval 1\nguide \"G\"\n");
        assert_eq!(
            rule_messages("oval 1\nguide \"G\"\n", Rule::SupportedVersion),
            [] as [String; 0]
        );
        assert!(
            report
                .diagnostics()
                .iter()
                .any(|d| d.code == DiagnosticCode::Syntax)
        );
    }

    #[test]
    fn documents_with_errors_do_not_compile() {
        let report = super::compile("guide \"G\"\n").expect_err("missing fields");
        assert!(report.has_errors());
    }

    #[test]
    fn compiled_json_matches_the_schema_conventions() {
        let source = "\
oval 0.1
guide \"G\"
election \"E\"
date 2026-11-03
party @dem \"Democratic\"
ballot @state \"State\"
  section \"Offices\"
    contest @gov \"Governor\" multiple 2
      campaign incumbent
        @j \"J\" as \"Governor\"
          party @dem
        @m \"M\" as \"Lieutenant Governor\"
          party @dem
      campaign @r \"R\"
      recommend
        support @j
        oppose @r
  ballot @city \"City\"
    measure @p \"P\"
      supporters none
      recommend no-position
";
        let compiled = super::compile(source).expect("compiles");
        let json: serde_json::Value = serde_json::from_str(&compiled.to_json().unwrap()).unwrap();
        let expected = serde_json::json!({
            "oval": "0.1",
            "guide": { "name": "G", "election": "E", "date": "2026-11-03" },
            "parties": [{ "id": "dem", "name": "Democratic", "candidates": ["j", "m"] }],
            "candidates": [
                { "id": "j", "name": "J", "party": "dem", "contests": ["gov"] },
                { "id": "m", "name": "M", "party": "dem", "contests": ["gov"] },
                { "id": "r", "name": "R", "contests": ["gov"] }
            ],
            "ballots": [{
                "kind": "ballot", "id": "state", "title": "State", "path": [],
                "items": [
                    { "kind": "section", "title": "Offices", "items": [{
                        "kind": "contest", "id": "gov", "title": "Governor",
                        "method": { "kind": "multiple", "seats": 2 },
                        "location": { "ballot": "state", "section": "Offices", "path": ["state"] },
                        "campaigns": [
                            {
                                "index": 0, "label": "J / M", "head": "j", "party": "dem",
                                "members": [
                                    { "candidate": "j", "role": "Governor" },
                                    { "candidate": "m", "role": "Lieutenant Governor" }
                                ],
                                "incumbent": true, "withdrawn": false,
                                "stance": { "position": "support", "tier": 0 }
                            },
                            {
                                "index": 1, "label": "R", "head": "r",
                                "members": [{ "candidate": "r" }],
                                "incumbent": false, "withdrawn": false,
                                "stance": { "position": "oppose" }
                            }
                        ],
                        "recommendation": {
                            "no_position": false,
                            "support": [["j"]], "oppose": ["r"],
                            "support_campaigns": [[0]], "oppose_campaigns": [1]
                        }
                    }]},
                    { "kind": "ballot", "id": "city", "title": "City", "path": ["state"], "items": [{
                        "kind": "measure", "id": "p", "title": "P",
                        "supporters": [],
                        "location": { "ballot": "city", "path": ["state", "city"] },
                        "recommendation": { "position": "no-position" }
                    }]}
                ]
            }],
            "questions": [
                { "kind": "contest", "id": "gov", "title": "Governor",
                  "location": { "ballot": "state", "section": "Offices", "path": ["state"] } },
                { "kind": "measure", "id": "p", "title": "P",
                  "location": { "ballot": "city", "path": ["state", "city"] } }
            ],
            "counts": { "ballots": 2, "sections": 1, "contests": 1, "measures": 1,
                        "campaigns": 2, "candidates": 3, "parties": 1 }
        });
        assert_eq!(json, expected);
        assert!(
            compiled
                .to_json_pretty()
                .unwrap()
                .contains("\n  \"guide\": {")
        );
    }
}
