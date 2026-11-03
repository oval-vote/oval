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
}
