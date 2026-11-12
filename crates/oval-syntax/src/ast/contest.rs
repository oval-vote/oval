use crate::ast::ast_node;

ast_node! {
    /// A contest, such as `contest @mayor "Mayor" single` and the block below it, in which voters
    /// choose among campaigns for an office.
    Contest
}

#[cfg(test)]
mod tests {
    use super::Contest;
    use crate::SyntaxKind;
    use crate::ast::AstNode;

    #[test]
    fn only_a_contest_node_is_a_contest() {
        assert!(Contest::can_cast(SyntaxKind::Contest));
        assert!(!Contest::can_cast(SyntaxKind::Measure));
    }
}
