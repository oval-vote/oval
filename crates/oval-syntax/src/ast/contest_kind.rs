use crate::ast::{AstNode, IntegerLiteral, ast_node, support};
use crate::{Keyword, SyntaxKind, SyntaxToken};

ast_node! {
    /// A contest's kind and the modifiers written after it, such as `ranked seats 3 max 6`.
    ///
    /// Each modifier is read as written. Whether its number is in range, and whether `all` is
    /// written alongside `max` or `min`, is left to validation. A modifier written twice is set
    /// aside by the parser, so its first occurrence is the one returned.
    ContestKind
}

/// How a contest is voted, the keyword that begins its [`ContestKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// `single`, a vote for one.
    Single,
    /// `multiple`, a vote for up to a number of choices.
    Multiple,
    /// `ranked`, a ranking of choices in order of preference.
    Ranked,
    /// `retention`, a yes or no on one candidate.
    Retention,
}

impl ContestKind {
    /// Returns how the contest is voted.
    ///
    /// # Panics
    ///
    /// Never, since the parser builds a contest kind only once it has read one of its four
    /// keywords.
    pub fn method(&self) -> Method {
        match support::keyword(self.syntax()) {
            Some(Keyword::Single) => Method::Single,
            Some(Keyword::Multiple) => Method::Multiple,
            Some(Keyword::Ranked) => Method::Ranked,
            Some(Keyword::Retention) => Method::Retention,
            other => unreachable!("a contest kind begins with its method, not {other:?}"),
        }
    }

    /// Returns the number of votes after `multiple`, or `None` if the contest is of another kind
    /// or the number is missing.
    pub fn votes(&self) -> Option<IntegerLiteral> {
        support::token(self.syntax())
    }

    /// Returns the number after `seats`, how many candidates are elected, or `None` if `seats` is
    /// not written or has no number.
    pub fn seats(&self) -> Option<IntegerLiteral> {
        self.modifier(SyntaxKind::Seats)
    }

    /// Returns the number after `min`, how many choices a ballot that marks the contest must mark,
    /// or `None` if `min` is not written or has no number.
    pub fn min(&self) -> Option<IntegerLiteral> {
        self.modifier(SyntaxKind::Minimum)
    }

    /// Returns the number after `max`, how many choices a voter may rank, or `None` if `max` is not
    /// written or has no number.
    pub fn max(&self) -> Option<IntegerLiteral> {
        self.modifier(SyntaxKind::RankedMax)
    }

    /// Returns the `all` keyword, which requires a ballot that marks the contest to rank every
    /// printed choice, or `None` if it is not written.
    pub fn all(&self) -> Option<SyntaxToken> {
        support::keyword_token(self.syntax(), Keyword::All)
    }

    fn modifier(&self, kind: SyntaxKind) -> Option<IntegerLiteral> {
        self.syntax()
            .children()
            .find(|child| child.kind() == kind)
            .and_then(|modifier| support::token(&modifier))
    }
}

#[cfg(test)]
mod tests {
    use super::{ContestKind, Method};
    use crate::SyntaxKind;
    use crate::ast::contest::first_contest;
    use crate::ast::{AstNode, IntegerLiteral};

    fn kind(header: &str) -> ContestKind {
        first_contest(&format!("  contest @m \"M\" {header}\n    campaign @a\n"))
            .kind()
            .expect("the contest has a kind")
    }

    fn number(integer: Option<IntegerLiteral>) -> Option<u64> {
        integer.and_then(|integer| integer.value())
    }

    #[test]
    fn each_keyword_is_a_method() {
        assert_eq!(kind("single").method(), Method::Single);
        assert_eq!(kind("multiple 2").method(), Method::Multiple);
        assert_eq!(kind("ranked").method(), Method::Ranked);
        assert_eq!(kind("retention").method(), Method::Retention);
    }

    #[test]
    fn a_kind_without_modifiers_has_none() {
        let kind = kind("single");
        assert_eq!(kind.votes(), None);
        assert_eq!(kind.seats(), None);
        assert_eq!(kind.min(), None);
        assert_eq!(kind.max(), None);
        assert_eq!(kind.all(), None);
    }

    #[test]
    fn multiple_has_its_votes_and_modifiers() {
        let kind = kind("multiple 5 seats 7 min 2");
        assert_eq!(number(kind.votes()), Some(5));
        assert_eq!(number(kind.seats()), Some(7));
        assert_eq!(number(kind.min()), Some(2));
    }

    #[test]
    fn ranked_has_its_modifiers_in_any_order() {
        let kind = kind("ranked min 2 max 6 seats 3");
        assert_eq!(number(kind.seats()), Some(3));
        assert_eq!(number(kind.max()), Some(6));
        assert_eq!(number(kind.min()), Some(2));
        assert_eq!(kind.votes(), None);
    }

    #[test]
    fn all_is_returned_as_its_keyword() {
        let all = kind("ranked all").all().expect("the kind has all");
        assert_eq!(all.text(), "all");
    }

    #[test]
    fn a_repeated_modifier_is_not_the_modifier() {
        assert_eq!(number(kind("ranked max 6 max 4").max()), Some(6));
    }

    #[test]
    fn a_modifier_without_a_number_has_none() {
        assert_eq!(kind("single seats").seats(), None);
        assert_eq!(kind("multiple").votes(), None);
    }

    #[test]
    fn a_modifier_the_kind_does_not_accept_is_not_read() {
        assert_eq!(kind("single max 5").max(), None);
    }

    #[test]
    fn only_a_contest_kind_node_is_a_contest_kind() {
        assert!(ContestKind::can_cast(SyntaxKind::ContestKind));
        assert!(!ContestKind::can_cast(SyntaxKind::Seats));
    }
}
