//! Resolution and validation for the Oval voting guide format.
//!
//! [`check`] parses a document and reports every problem in it as a [`Diagnostic`], ordered by
//! where each one begins.
//!
//! ```
//! use oval_core::Severity;
//!
//! let report = oval_core::check("guide 5\n")?;
//! assert!(report.has_errors());
//!
//! let diagnostic = &report.diagnostics()[0];
//! assert_eq!(diagnostic.code.spec_id(), "unexpected-token");
//! assert_eq!(diagnostic.severity, Severity::Error);
//! assert_eq!(&"guide 5\n"[diagnostic.range], "5");
//! # Ok::<(), oval_core::InputRejection>(())
//! ```

mod diagnostic;
mod report;

pub use diagnostic::{Diagnostic, DiagnosticCode, Severity};
pub use oval_syntax::InputRejection;
pub use report::Report;

/// Parses a document and reports every problem found in it.
///
/// The diagnostics are ordered by where each one begins. When a lexical and a syntax problem begin
/// at the same place, the lexical one comes first.
///
/// An [`InputRejection`] means the input cannot be an Oval document at all, because it contains
/// U+0000, is not UTF-8, or is too large.
pub fn check(text: &str) -> Result<Report, InputRejection> {
    let parse = oval_syntax::parse(text)?;
    let mut diagnostics: Vec<Diagnostic> = parse
        .lexical_errors
        .iter()
        .map(Diagnostic::from)
        .chain(parse.syntax_errors.iter().map(Diagnostic::from))
        .collect();
    diagnostics.sort_by_key(|diagnostic| diagnostic.range.start());
    Ok(Report::new(parse, diagnostics))
}

#[cfg(test)]
mod tests {
    use oval_syntax::{LexicalErrorKind, SyntaxErrorKind, TextRange};

    use super::{DiagnosticCode, check};

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
        let report = check("oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\n").unwrap();
        assert_eq!(report.diagnostics(), []);
        assert!(!report.has_errors());
    }

    #[test]
    fn a_lexical_error_is_a_diagnostic() {
        assert_eq!(
            codes("guide \"a\\#b\"\n"),
            [(
                DiagnosticCode::Lexical(LexicalErrorKind::InvalidEscape),
                range(8, 10)
            )]
        );
    }

    #[test]
    fn a_syntax_error_is_a_diagnostic() {
        assert_eq!(
            codes("guide 5\n"),
            [(
                DiagnosticCode::Syntax(SyntaxErrorKind::UnexpectedToken),
                range(6, 7)
            )]
        );
    }

    #[test]
    fn diagnostics_are_ordered_by_where_they_begin() {
        assert_eq!(
            codes("guide 5\nelection \"a\\#b\"\n"),
            [
                (
                    DiagnosticCode::Syntax(SyntaxErrorKind::UnexpectedToken),
                    range(6, 7)
                ),
                (
                    DiagnosticCode::Lexical(LexicalErrorKind::InvalidEscape),
                    range(19, 21)
                ),
            ]
        );
    }

    #[test]
    fn a_lexical_error_comes_before_a_syntax_error_at_the_same_place() {
        assert_eq!(
            codes("guide \"G\"\n  ~ x\n"),
            [
                (
                    DiagnosticCode::Lexical(LexicalErrorKind::UnexpectedCharacter),
                    range(12, 13)
                ),
                (
                    DiagnosticCode::Syntax(SyntaxErrorKind::UnexpectedBlock),
                    range(12, 15)
                ),
            ]
        );
    }

    #[test]
    fn the_report_keeps_the_syntax_tree() {
        let report = check("guide \"G\"\n").unwrap();
        assert_eq!(report.parse().syntax().text(), "guide \"G\"\n");
    }

    #[test]
    fn input_that_cannot_be_a_document_is_rejected() {
        assert!(check("guide \"G\0\"\n").is_err());
    }
}
