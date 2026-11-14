use oval_syntax::ParseResult;

use crate::{Diagnostic, Severity};

/// The result of [`check`](crate::check)ing a document, its syntax tree and every problem found in
/// it.
#[derive(Debug)]
pub struct Report {
    parse: ParseResult,
    diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub(crate) fn new(parse: ParseResult, diagnostics: Vec<Diagnostic>) -> Self {
        Self { parse, diagnostics }
    }

    /// Returns every problem found in the document, ordered by where each one begins.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns `true` if any problem is an [`Error`](Severity::Error), which means the document is
    /// not a valid Oval document. Warnings alone leave it valid.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }

    /// Returns the result of parsing the document, which holds its syntax tree.
    pub fn parse(&self) -> &ParseResult {
        &self.parse
    }
}
