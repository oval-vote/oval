use crate::ast::ast_node;

ast_node! {
    /// A ballot measure, such as `measure @prop1 "Proposition 1"` and the block below it, which
    /// voters answer yes or no.
    Measure
}

#[cfg(test)]
mod tests {
    use super::Measure;
    use crate::SyntaxKind;
    use crate::ast::AstNode;

    #[test]
    fn only_a_measure_node_is_a_measure() {
        assert!(Measure::can_cast(SyntaxKind::Measure));
        assert!(!Measure::can_cast(SyntaxKind::Contest));
    }
}
