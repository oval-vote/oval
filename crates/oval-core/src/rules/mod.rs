mod identifiers;

use crate::Diagnostic;
use crate::index::Index;

pub(crate) fn check(index: &Index) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    identifiers::check(index, &mut diagnostics);
    diagnostics
}
