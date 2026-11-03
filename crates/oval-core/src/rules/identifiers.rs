use crate::index::Index;
use crate::{Diagnostic, DiagnosticCode, Rule, Severity};

pub(crate) fn check(index: &Index, out: &mut Vec<Diagnostic>) {
    for i in &index.duplicates {
        let declaration = &index.declarations[*i];
        let first = index
            .resolve(&declaration.name)
            .expect("duplicate has a first declaration");
        out.push(error(
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
                reference.range,
                format!("`@{}` is not declared", reference.name),
            )),
            Some(declaration) if declaration.kind != reference.expected => out.push(error(
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

fn error(range: oval_syntax::TextRange, message: String) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::Rule(Rule::Identifiers),
        severity: Severity::Error,
        range,
        message,
    }
}
