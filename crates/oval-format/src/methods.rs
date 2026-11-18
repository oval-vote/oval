use oval_syntax::ast::{self, AstNode, AstToken, IntegerLiteral, Method};
use oval_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

pub(crate) fn ordered(kind: &SyntaxNode) -> Vec<SyntaxElement> {
    let Some(contest_kind) = ast::ContestKind::cast(kind.clone()) else {
        return kind.children_with_tokens().collect();
    };
    let method = contest_kind.method();
    let votes = contest_kind.votes().and_then(|votes| votes.value());
    let mut elements = Vec::new();
    let mut modifiers = Vec::new();
    for element in kind.children_with_tokens() {
        match rank(&element) {
            Some(_) if is_default(&element, method, votes) => {}
            Some(rank) => modifiers.push((rank, element)),
            None => elements.push(element),
        }
    }
    modifiers.sort_by_key(|(rank, _)| *rank);
    elements.extend(modifiers.into_iter().map(|(_, element)| element));
    elements
}

fn rank(element: &SyntaxElement) -> Option<u8> {
    match element {
        SyntaxElement::Node(node) => match node.kind() {
            SyntaxKind::Seats => Some(0),
            SyntaxKind::RankedMax => Some(1),
            SyntaxKind::Minimum => Some(2),
            _ => None,
        },
        SyntaxElement::Token(token) => {
            (token.kind() == SyntaxKind::Word && token.text() == "all").then_some(3)
        }
    }
}

fn is_default(element: &SyntaxElement, method: Method, votes: Option<u64>) -> bool {
    let SyntaxElement::Node(node) = element else {
        return false;
    };
    let value = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find_map(IntegerLiteral::cast)
        .and_then(|integer| integer.value());
    match node.kind() {
        SyntaxKind::Seats if method == Method::Multiple => value.is_some() && value == votes,
        SyntaxKind::Seats | SyntaxKind::Minimum => value == Some(1),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::format;

    fn header(kind: &str) -> String {
        let text = format!(
            "oval 1\nballot @b \"B\"\n  contest @k \"K\" {kind}\n    recommend no-position\n"
        );
        let formatted = format(&text).unwrap();
        assert_eq!(format(&formatted).unwrap(), formatted, "formats to itself");
        let line = formatted.lines().nth(2).unwrap();
        line["  contest @k \"K\" ".len()..].to_owned()
    }

    #[test]
    fn a_modifier_that_states_its_default_is_removed() {
        assert_eq!(header("ranked min 1 seats 1 max 6"), "ranked max 6");
        assert_eq!(header("multiple 3 min 2 seats 3"), "multiple 3 min 2");
        assert_eq!(header("single seats 1"), "single");
        assert_eq!(header("single seats 01"), "single");
        assert_eq!(header("multiple 03 seats 3 min 01"), "multiple 3");
        assert_eq!(header("ranked all min 1"), "ranked all");
    }

    #[test]
    fn a_modifier_that_differs_from_its_default_is_kept() {
        assert_eq!(header("single seats 2"), "single seats 2");
        assert_eq!(header("multiple 3 seats 2"), "multiple 3 seats 2");
        assert_eq!(header("multiple 3 seats 1"), "multiple 3 seats 1");
        assert_eq!(header("ranked seats 3 min 2"), "ranked seats 3 min 2");
        assert_eq!(
            header("single seats 99999999999999999999"),
            "single seats 99999999999999999999"
        );
    }

    #[test]
    fn modifiers_are_written_as_seats_max_min_then_all() {
        assert_eq!(
            header("ranked all min 2 max 6 seats 2"),
            "ranked seats 2 max 6 min 2 all"
        );
        assert_eq!(
            header("multiple 4 min 2 seats 3"),
            "multiple 4 seats 3 min 2"
        );
        assert_eq!(header("ranked max 4 all"), "ranked max 4 all");
    }
}
