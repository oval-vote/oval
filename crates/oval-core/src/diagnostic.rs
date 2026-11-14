use oval_syntax::{LexicalError, LexicalErrorKind, SyntaxError, SyntaxErrorKind, TextRange};

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// The document is not a valid Oval document.
    Error,
    /// The document is valid, but something in it is probably a mistake.
    Warning,
}

/// The kind of problem a [`Diagnostic`] reports, grouped by the stage that finds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// A problem found while splitting the document into tokens.
    Lexical(LexicalErrorKind),
    /// A problem found while matching the tokens against the grammar.
    Syntax(SyntaxErrorKind),
}

impl DiagnosticCode {
    /// Returns the identifier the specification gives this problem, such as `missing-value`.
    pub fn spec_id(self) -> &'static str {
        match self {
            Self::Lexical(kind) => kind.spec_id(),
            Self::Syntax(kind) => kind.spec_id(),
        }
    }

    /// Returns how serious this problem is. Every lexical and syntax problem is an error.
    pub fn severity(self) -> Severity {
        match self {
            Self::Lexical(_) | Self::Syntax(_) => Severity::Error,
        }
    }
}

/// A problem found in a document, and the text it spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The kind of problem.
    pub code: DiagnosticCode,
    /// How serious the problem is.
    pub severity: Severity,
    /// The bytes of the document the problem spans. A missing token spans no bytes, and sits where
    /// the token would have begun.
    pub range: TextRange,
}

impl Diagnostic {
    fn new(code: DiagnosticCode, offset: u32, len: u32) -> Self {
        Self {
            code,
            severity: code.severity(),
            range: TextRange::at(offset.into(), len.into()),
        }
    }
}

impl From<&LexicalError> for Diagnostic {
    fn from(error: &LexicalError) -> Self {
        Self::new(DiagnosticCode::Lexical(error.kind), error.offset, error.len)
    }
}

impl From<&SyntaxError> for Diagnostic {
    fn from(error: &SyntaxError) -> Self {
        Self::new(DiagnosticCode::Syntax(error.kind), error.offset, error.len)
    }
}

#[cfg(test)]
mod tests {
    use oval_syntax::{LexicalError, LexicalErrorKind, SyntaxError, SyntaxErrorKind, TextRange};

    use super::{Diagnostic, DiagnosticCode, Severity};

    #[test]
    fn a_code_has_the_identifier_of_its_kind() {
        let lexical = DiagnosticCode::Lexical(LexicalErrorKind::InvalidEscape);
        let syntax = DiagnosticCode::Syntax(SyntaxErrorKind::MissingValue);
        assert_eq!(lexical.spec_id(), "invalid-escape");
        assert_eq!(syntax.spec_id(), "missing-value");
    }

    #[test]
    fn lexical_and_syntax_problems_are_errors() {
        let lexical = DiagnosticCode::Lexical(LexicalErrorKind::InvalidEscape);
        let syntax = DiagnosticCode::Syntax(SyntaxErrorKind::MissingValue);
        assert_eq!(lexical.severity(), Severity::Error);
        assert_eq!(syntax.severity(), Severity::Error);
    }

    #[test]
    fn a_lexical_error_becomes_a_diagnostic_over_the_same_bytes() {
        let error = LexicalError {
            kind: LexicalErrorKind::InvalidEscape,
            offset: 8,
            len: 2,
        };
        let diagnostic = Diagnostic::from(&error);
        assert_eq!(
            diagnostic.code,
            DiagnosticCode::Lexical(LexicalErrorKind::InvalidEscape)
        );
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.range, TextRange::new(8.into(), 10.into()));
    }

    #[test]
    fn a_syntax_error_becomes_a_diagnostic_over_the_same_bytes() {
        let error = SyntaxError {
            kind: SyntaxErrorKind::MissingValue,
            offset: 5,
            len: 0,
            expected: Vec::new(),
        };
        let diagnostic = Diagnostic::from(&error);
        assert_eq!(
            diagnostic.code,
            DiagnosticCode::Syntax(SyntaxErrorKind::MissingValue)
        );
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.range, TextRange::empty(5.into()));
    }
}
