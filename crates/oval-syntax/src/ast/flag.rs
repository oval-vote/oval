use crate::Keyword;
use crate::ast::{AstNode, ast_node, support};

ast_node! {
    /// A campaign's flag, `incumbent` or `withdrawn`, written after `campaign`.
    Flag
}

impl Flag {
    /// Returns `true` if the flag is `incumbent`, which means the campaign holds the office the
    /// contest is for.
    pub fn is_incumbent(&self) -> bool {
        support::keyword(self.syntax()) == Some(Keyword::Incumbent)
    }

    /// Returns `true` if the flag is `withdrawn`, which means the campaign has ended but is still
    /// on the ballot.
    pub fn is_withdrawn(&self) -> bool {
        support::keyword(self.syntax()) == Some(Keyword::Withdrawn)
    }
}

#[cfg(test)]
mod tests {
    use super::Flag;
    use crate::SyntaxKind;
    use crate::ast::AstNode;
    use crate::ast::campaign::first_campaign;

    fn flag(word: &str) -> Flag {
        first_campaign(&format!("    campaign {word} @a\n"))
            .flags()
            .next()
            .expect("the campaign has a flag")
    }

    #[test]
    fn incumbent_is_incumbent() {
        let flag = flag("incumbent");
        assert!(flag.is_incumbent());
        assert!(!flag.is_withdrawn());
    }

    #[test]
    fn withdrawn_is_withdrawn() {
        let flag = flag("withdrawn");
        assert!(flag.is_withdrawn());
        assert!(!flag.is_incumbent());
    }

    #[test]
    fn only_a_flag_node_is_a_flag() {
        assert!(Flag::can_cast(SyntaxKind::Flag));
        assert!(!Flag::can_cast(SyntaxKind::Requirement));
    }
}
