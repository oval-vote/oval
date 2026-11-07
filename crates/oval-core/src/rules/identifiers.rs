use super::report;
use crate::index::Index;
use crate::{Diagnostic, ValidationRule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for i in &index.duplicates {
        let declaration = &index.declarations[*i];
        let first = index
            .resolve(&declaration.name)
            .expect("duplicate has a first declaration");
        out.push(report(
            ValidationRule::DuplicateDeclaration,
            declaration.range,
            format!(
                "`@{}` is already declared as a {}",
                declaration.name,
                first.kind.noun()
            ),
        ));
    }
    for reference in &index.references {
        match index.resolve(&reference.name) {
            None => out.push(report(
                ValidationRule::UndeclaredReference,
                reference.range,
                format!("`@{}` is not declared", reference.name),
            )),
            Some(declaration) if declaration.kind != reference.expected => out.push(report(
                ValidationRule::MismatchedReference,
                reference.range,
                format!(
                    "`@{}` is a {}, not a {}",
                    reference.name,
                    declaration.kind.noun(),
                    reference.expected.noun()
                ),
            )),
            Some(_) => {}
        }
    }
}
