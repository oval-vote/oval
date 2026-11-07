use oval_syntax::ast::AstNode;

use super::report;
use crate::index::Index;
use crate::{Diagnostic, ValidationRule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for measure in &index.measures {
        match (measure.if_yes(), measure.if_no()) {
            (Some(present), None) => out.push(report(
                ValidationRule::UnpairedOutcome,
                present.range(),
                "`if_yes` is present without `if_no`; the ballot prints both or neither".to_owned(),
            )),
            (None, Some(present)) => out.push(report(
                ValidationRule::UnpairedOutcome,
                present.range(),
                "`if_no` is present without `if_yes`; the ballot prints both or neither".to_owned(),
            )),
            _ => {}
        }
    }
}
