use oval_syntax::TextRange;
use oval_syntax::ast::Document;

use super::error;
use crate::{Diagnostic, Rule};

pub(crate) fn check(document: &Document, out: &mut Vec<Diagnostic>) {
    let missing = [
        ("guide", document.guide().is_none()),
        ("election", document.election().is_none()),
        ("date", document.date().is_none()),
    ];
    for (name, is_missing) in missing {
        if is_missing {
            out.push(error(
                Rule::RequiredGuideFields,
                TextRange::empty(0.into()),
                format!("the document has no `{name}` field"),
            ));
        }
    }
}
