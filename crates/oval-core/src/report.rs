use oval_syntax::ParseResult;

use crate::model::Compiled;
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

    /// Compiles the document to its canonical output, or returns `None` if the report
    /// [`has_errors`](Self::has_errors).
    ///
    /// Warnings don't prevent compiling. When the document doesn't compile, its errors are in
    /// [`diagnostics`](Self::diagnostics).
    ///
    /// ```
    /// let report = oval_core::check("oval 1\nguide \"Franklin Voter Guide\"\n")?;
    /// assert!(report.compile().is_none());
    /// # Ok::<(), oval_core::InputRejection>(())
    /// ```
    pub fn compile(&self) -> Option<Compiled> {
        if self.has_errors() {
            return None;
        }
        crate::compile::compile(&self.parse.document())
    }
}

#[cfg(test)]
mod tests {
    use crate::check;
    use crate::validate::VALID;

    #[test]
    fn a_document_with_an_error_does_not_compile() {
        let report = check(&VALID.replacen("date 2026-11-03", "date 2026-02-30", 1)).unwrap();
        assert!(report.has_errors());
        assert_eq!(report.compile(), None);
    }

    #[test]
    fn a_document_with_only_warnings_compiles() {
        let report = check(&VALID.replacen("\"E\"", "\"\"", 1)).unwrap();
        assert_ne!(report.diagnostics(), []);
        assert!(!report.has_errors());
        assert_eq!(report.compile().unwrap().guide.election, "");
    }
}
