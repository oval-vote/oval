//! Resolution and validation for the Oval voting guide format.
//!
//! [`check`] parses a document, validates it, and reports every problem in it as a [`Diagnostic`],
//! ordered by where each one begins.
//!
//! ```
//! use oval_core::{DiagnosticCode, ValidationRule};
//!
//! let report = oval_core::check("oval 1\nguide \"Franklin Voter Guide\"\n")?;
//! assert!(report.has_errors());
//!
//! let codes: Vec<&str> = report
//!     .diagnostics()
//!     .iter()
//!     .map(|diagnostic| diagnostic.code.spec_id())
//!     .collect();
//! assert_eq!(
//!     codes,
//!     ["missing-guide-field", "missing-guide-field", "empty-document"]
//! );
//! assert_eq!(
//!     report.diagnostics()[2].code,
//!     DiagnosticCode::Validation(ValidationRule::EmptyDocument)
//! );
//! # Ok::<(), oval_core::InputRejection>(())
//! ```

mod compile;
mod diagnostic;
pub mod model;
mod report;
mod rule;
mod validate;

pub use diagnostic::{Diagnostic, DiagnosticCode, Severity};
pub use oval_syntax::InputRejection;
pub use report::Report;
pub use rule::ValidationRule;

/// Parses and validates a document, and reports every problem found in it.
///
/// The diagnostics are ordered by where each one begins. When a lexical and a syntax problem begin
/// at the same place, the lexical one comes first, and a broken rule comes after both.
///
/// A document whose version is missing or unsupported is reported with a single
/// [`UnsupportedVersion`](ValidationRule::UnsupportedVersion) diagnostic and nothing else, not even
/// its lexical and syntax errors, since the rest of it may not mean what this version of the
/// language says.
///
/// An [`InputRejection`] means the input cannot be an Oval document at all, because it contains
/// U+0000, is not UTF-8, or is too large.
pub fn check(text: &str) -> Result<Report, InputRejection> {
    let parse = oval_syntax::parse(text)?;
    let diagnostics = match validate::validate(&parse.document()) {
        Ok(broken_rules) => {
            let mut diagnostics: Vec<Diagnostic> = parse
                .lexical_errors
                .iter()
                .map(Diagnostic::from)
                .chain(parse.syntax_errors.iter().map(Diagnostic::from))
                .chain(broken_rules)
                .collect();
            diagnostics.sort_by_key(|diagnostic| diagnostic.range.start());
            diagnostics
        }
        Err(unsupported_version) => vec![unsupported_version],
    };
    Ok(Report::new(parse, diagnostics))
}

#[cfg(test)]
mod tests {
    use oval_syntax::{LexicalErrorKind, SyntaxErrorKind, TextRange};

    use super::{DiagnosticCode, check};
    use crate::validate::VALID;

    fn codes(text: &str) -> Vec<(DiagnosticCode, TextRange)> {
        check(text)
            .unwrap()
            .diagnostics()
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.range))
            .collect()
    }

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    #[test]
    fn a_document_without_problems_has_no_diagnostics() {
        let report = check(VALID).unwrap();
        assert_eq!(report.diagnostics(), []);
        assert!(!report.has_errors());
    }

    #[test]
    fn a_lexical_error_is_a_diagnostic() {
        assert_eq!(
            codes(&VALID.replacen("\"G\"", "\"a\\#b\"", 1)),
            [(
                DiagnosticCode::Lexical(LexicalErrorKind::InvalidEscape),
                range(15, 17)
            )]
        );
    }

    #[test]
    fn a_syntax_error_is_a_diagnostic() {
        assert_eq!(
            codes(&VALID.replacen("\"G\"", "5", 1)),
            [(
                DiagnosticCode::Syntax(SyntaxErrorKind::UnexpectedToken),
                range(13, 14)
            )]
        );
    }

    #[test]
    fn diagnostics_are_ordered_by_where_they_begin() {
        let text = VALID
            .replacen("\"G\"", "5", 1)
            .replacen("\"E\"", "\"a\\#b\"", 1);
        assert_eq!(
            codes(&text),
            [
                (
                    DiagnosticCode::Syntax(SyntaxErrorKind::UnexpectedToken),
                    range(13, 14)
                ),
                (
                    DiagnosticCode::Lexical(LexicalErrorKind::InvalidEscape),
                    range(26, 28)
                ),
            ]
        );
    }

    #[test]
    fn a_lexical_error_comes_before_a_syntax_error_at_the_same_place() {
        let head = &VALID[..VALID.find("ballot").unwrap()];
        let text = format!("{head}ballot @b \"B\"\n  section \"S\"\n contest @c \"C\" single\n");
        assert_eq!(
            codes(&text),
            [
                (
                    DiagnosticCode::Lexical(LexicalErrorKind::InconsistentDedent),
                    range(74, 75)
                ),
                (
                    DiagnosticCode::Syntax(SyntaxErrorKind::MissingBlock),
                    range(74, 74)
                ),
                (
                    DiagnosticCode::Syntax(SyntaxErrorKind::UnexpectedEntry),
                    range(75, 96)
                ),
            ]
        );
    }

    #[test]
    fn the_report_keeps_the_syntax_tree() {
        let report = check(VALID).unwrap();
        assert_eq!(report.parse().syntax().text(), VALID);
    }

    #[test]
    fn input_that_cannot_be_a_document_is_rejected() {
        assert!(check("oval 1\nguide \"G\0\"\n").is_err());
    }
}
