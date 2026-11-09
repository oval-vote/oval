//! The validation rules of Sections 3.3 and 3.4 of the specification, one module per group.

mod campaigns;
mod contests;
mod dates;
mod declarations;
mod guide;
mod identifiers;
mod markdown;
mod measures;
mod questions;
mod recommendations;
mod strings;
mod structure;
mod urls;
pub(crate) mod version;

use oval_syntax::TextRange;
use oval_syntax::ast::Document;

use crate::index::Index;
use crate::{Diagnostic, DiagnosticCode, ValidationRule};

/// Runs every rule over `document` and returns the diagnostics, in the order the rules ran.
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
    markdown::check(document, &mut diagnostics);
    urls::check(document, &mut diagnostics);
    strings::check(document, &mut diagnostics);
    diagnostics
}

/// A diagnostic for `rule` at `range`, with the severity the rule carries.
fn report(rule: ValidationRule, range: TextRange, message: String) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::Validation(rule), range, message)
}
