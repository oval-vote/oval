use oval_syntax::ast::{AstNode, Document, UrlField};

use super::report;
use crate::{Diagnostic, ValidationRule};

pub(crate) fn check(document: &Document, out: &mut Vec<Diagnostic>) {
    for field in document.syntax().descendants().filter_map(UrlField::cast) {
        let Some(value) = field.value() else {
            continue;
        };
        let Some(range) = field.value_range() else {
            continue;
        };
        if value.starts_with("http://") {
            out.push(report(
                ValidationRule::InsecureUrl,
                range,
                "this URL uses `http`; use `https`".to_owned(),
            ));
        }
    }
}
