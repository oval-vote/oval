mod campaigns;
mod contests;
mod dates;
mod declarations;
mod guide;
mod identifiers;
mod measures;
mod questions;
mod recommendations;
mod structure;
pub(crate) mod version;

use oval_syntax::TextRange;
use oval_syntax::ast::Document;

use crate::index::Index;
use crate::{Diagnostic, DiagnosticCode, ValidationRule};

pub(crate) fn check(document: &Document, index: &Index) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    identifiers::check(index, &mut diagnostics);
    guide::check(document, &mut diagnostics);
    campaigns::check(index, &mut diagnostics);
    contests::check(index, &mut diagnostics);
    recommendations::check(index, &mut diagnostics);
    questions::check(index, &mut diagnostics);
    structure::check(document, index, &mut diagnostics);
    dates::check(document, &mut diagnostics);
    declarations::check(document, index, &mut diagnostics);
    measures::check(index, &mut diagnostics);
    diagnostics
}

fn report(rule: ValidationRule, range: TextRange, message: String) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::Validation(rule), range, message)
}
