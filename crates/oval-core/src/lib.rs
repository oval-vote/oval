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
        diagnostics.extend(rules::check(&index));
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
        let report = check("guide");
        assert_eq!(
            report.diagnostics()[0].message,
            "expected a string, found the end of the line"
        );
        let report = check("ballot @b \"B\"");
        assert_eq!(
            report.diagnostics()[0].message,
            "expected an indented block, found the end of the file"
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
        let range = report.diagnostics()[0].range;
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
}
