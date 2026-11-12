use crate::ast::{AstNode, Choice, ast_node, support};
use crate::{SyntaxKind, SyntaxNode};

ast_node! {
    /// A `support` line in a recommendation, one tier of choices the guide recommends, such as
    /// `support @aokafor @bsantos`.
    ///
    /// Several `support` lines are several tiers, from most to least preferred. The choices within
    /// one line are equally preferred.
    Support
}

ast_node! {
    /// An `oppose` line in a recommendation, the choices the guide advises against, such as
    /// `oppose @cfox`.
    Oppose
}

impl Support {
    /// Returns the line's choices, in the order written.
    pub fn choices(&self) -> impl Iterator<Item = Choice> + use<> {
        support::children(self.syntax())
    }
}

impl Oppose {
    /// Returns the line's choices, in the order written.
    pub fn choices(&self) -> impl Iterator<Item = Choice> + use<> {
        support::children(self.syntax())
    }
}

/// A line of a recommendation that takes a stance on choices, `support` or `oppose`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stance {
    /// A `support` line.
    Support(Support),
    /// An `oppose` line.
    Oppose(Oppose),
}

impl Stance {
    /// Returns the line's choices, in the order written.
    pub fn choices(&self) -> impl Iterator<Item = Choice> + use<> {
        support::children(self.syntax())
    }
}

impl AstNode for Stance {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::Support | SyntaxKind::Oppose)
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Support => Support::cast(node).map(Self::Support),
            SyntaxKind::Oppose => Oppose::cast(node).map(Self::Oppose),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Support(support) => support.syntax(),
            Self::Oppose(oppose) => oppose.syntax(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Oppose, Stance, Support};
    use crate::SyntaxKind;
    use crate::ast::contest_recommend::first_recommendation;
    use crate::ast::{AstNode, AstToken, Choice};

    fn describe(choices: impl Iterator<Item = Choice>) -> Vec<String> {
        choices
            .map(|choice| match choice {
                Choice::Reference(identifier) => identifier
                    .name()
                    .map_or_else(String::new, |name| format!("@{}", name.text())),
                Choice::WriteIn(write_in) => write_in.name().map_or_else(
                    || "write-in".to_owned(),
                    |name| format!("write-in {}", name.text()),
                ),
            })
            .collect()
    }

    #[test]
    fn a_support_line_has_its_choices_in_order() {
        let recommendation = first_recommendation(
            "    recommend\n      support @bsantos write-in \"Hiro Eriksen\" @dwright\n",
        );
        let support = recommendation.supports().next().expect("a support line");
        assert_eq!(
            describe(support.choices()),
            ["@bsantos", "write-in \"Hiro Eriksen\"", "@dwright"]
        );
    }

    #[test]
    fn an_oppose_line_has_its_choices() {
        let recommendation = first_recommendation("    recommend\n      oppose @cfox write-in\n");
        let oppose = recommendation.opposes().next().expect("an oppose line");
        assert_eq!(describe(oppose.choices()), ["@cfox", "write-in"]);
    }

    #[test]
    fn a_stance_has_the_choices_of_its_line() {
        let recommendation = first_recommendation("    recommend\n      oppose @c\n");
        let stance = recommendation.stances().next().expect("a stance");
        assert!(matches!(stance, Stance::Oppose(_)));
        assert_eq!(describe(stance.choices()), ["@c"]);
    }

    #[test]
    fn a_choice_skipped_by_the_parser_is_not_a_choice() {
        let recommendation = first_recommendation("    recommend\n      support @a 5 @b\n");
        let support = recommendation.supports().next().expect("a support line");
        assert_eq!(describe(support.choices()), ["@a"]);
    }

    #[test]
    fn a_stance_is_a_support_or_oppose_line() {
        assert!(Stance::can_cast(SyntaxKind::Support));
        assert!(Stance::can_cast(SyntaxKind::Oppose));
        assert!(!Stance::can_cast(SyntaxKind::NoStance));
        assert!(Support::can_cast(SyntaxKind::Support));
        assert!(!Oppose::can_cast(SyntaxKind::Support));
    }
}
