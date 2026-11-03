mod campaigns;
mod guide;
mod identifiers;
mod recommendations;

use oval_syntax::TextRange;
use oval_syntax::ast::Document;

use crate::index::Index;
use crate::{Diagnostic, DiagnosticCode, Rule, Severity};

pub(crate) fn check(document: &Document, index: &Index) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    identifiers::check(index, &mut diagnostics);
    guide::check(document, &mut diagnostics);
    campaigns::check(index, &mut diagnostics);
    recommendations::check(index, &mut diagnostics);
    diagnostics
}

fn error(rule: Rule, range: TextRange, message: String) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::Rule(rule),
        severity: Severity::Error,
        range,
        message,
    }
}
