//! Resolution and validation for the Oval voting guide format.

mod diagnostic;
mod syntax;

pub use diagnostic::{Diagnostic, DiagnosticCode, Rule, Severity};

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
    diagnostics.sort_by_key(|d| d.range.start());
    Report { parse, diagnostics }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticCode, Severity, check};

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
}
