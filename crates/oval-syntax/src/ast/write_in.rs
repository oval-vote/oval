use crate::ast::{AstNode, StringLiteral, ast_node, support};

ast_node! {
    /// A write-in choice, `write-in` and the name of the person to write in, if it has one, such
    /// as `write-in "Jane Doe"`.
    ///
    /// A write-in with no name names no one. In a `support` line it means writing in anyone the
    /// reader chooses, and in an `oppose` line it means writing in no one.
    WriteIn
}

impl WriteIn {
    /// Returns the name to write in, or `None` if the write-in names no one.
    pub fn name(&self) -> Option<StringLiteral> {
        support::token(self.syntax())
    }
}

#[cfg(test)]
mod tests {
    use super::WriteIn;
    use crate::SyntaxKind;
    use crate::ast::contest_recommend::first_recommendation;
    use crate::ast::{AstNode, Choice};

    fn write_ins(line: &str) -> Vec<WriteIn> {
        first_recommendation(&format!("    recommend\n      {line}\n"))
            .stances()
            .flat_map(|stance| stance.choices().collect::<Vec<_>>())
            .filter_map(|choice| match choice {
                Choice::WriteIn(write_in) => Some(write_in),
                Choice::Reference(_) => None,
            })
            .collect()
    }

    #[test]
    fn a_write_in_can_have_a_name() {
        let write_ins = write_ins("support write-in \"Jane Doe\"");
        let name = write_ins[0].name().map(|name| name.value().into_owned());
        assert_eq!(name.as_deref(), Some("Jane Doe"));
    }

    #[test]
    fn a_bare_write_in_has_no_name() {
        assert_eq!(write_ins("oppose @c write-in")[0].name(), None);
    }

    #[test]
    fn only_a_write_in_node_is_a_write_in() {
        assert!(WriteIn::can_cast(SyntaxKind::WriteIn));
        assert!(!WriteIn::can_cast(SyntaxKind::Identifier));
    }
}
