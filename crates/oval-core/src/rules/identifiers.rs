use super::error;
use crate::index::Index;
use crate::{Diagnostic, Rule};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for i in &index.duplicates {
        let declaration = &index.declarations[*i];
        let first = index
            .resolve(&declaration.name)
            .expect("duplicate has a first declaration");
        out.push(error(
            Rule::Identifiers,
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
            None => out.push(error(
                Rule::Identifiers,
                reference.range,
                format!("`@{}` is not declared", reference.name),
            )),
            Some(declaration) if declaration.kind != reference.expected => out.push(error(
                Rule::Identifiers,
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
