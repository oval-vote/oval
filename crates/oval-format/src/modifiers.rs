use oval_syntax::ast::{self, AstNode, Extensibility, Requirement};
use oval_syntax::{SyntaxKind, SyntaxNode};

pub(crate) fn is_redundant(node: &SyntaxNode) -> bool {
    let Some(construct) = node.parent() else {
        return false;
    };
    if let Some(requirement) = Requirement::cast(node.clone()) {
        requirement.is_required()
            == inherited::<Requirement>(&construct).is_some_and(|parent| parent.is_required())
    } else if let Some(extensibility) = Extensibility::cast(node.clone()) {
        if is_retention(&construct) {
            !extensibility.is_extensible()
        } else {
            extensibility.is_extensible()
                == inherited::<Extensibility>(&construct)
                    .is_some_and(|parent| parent.is_extensible())
        }
    } else {
        false
    }
}

fn inherited<N: AstNode>(construct: &SyntaxNode) -> Option<N> {
    construct
        .ancestors()
        .skip(1)
        .filter(|ancestor| matches!(ancestor.kind(), SyntaxKind::Section | SyntaxKind::Ballot))
        .find_map(|ancestor| ancestor.children().find_map(N::cast))
}

fn is_retention(construct: &SyntaxNode) -> bool {
    ast::Contest::cast(construct.clone())
        .and_then(|contest| contest.kind())
        .is_some_and(|kind| kind.method() == ast::Method::Retention)
}

#[cfg(test)]
mod tests {
    use crate::format;

    fn formatted(text: &str) -> String {
        let formatted = format(text).unwrap();
        assert_eq!(format(&formatted).unwrap(), formatted, "formats to itself");
        formatted
    }

    #[test]
    fn a_modifier_that_states_what_its_construct_would_inherit_is_removed() {
        assert_eq!(
            formatted(
                "oval 1\noptional exhaustive ballot @city \"City of Ashford\"\n  required extensible section \"City Offices\"\n    required extensible contest @mayor \"Mayor\" single\n      recommend no-position\n    optional exhaustive contest @auditor \"City Auditor\" single\n      recommend no-position\n"
            ),
            "oval 1\nballot @city \"City of Ashford\"\n  required extensible section \"City Offices\"\n    contest @mayor \"Mayor\" single\n      recommend no-position\n    optional exhaustive contest @auditor \"City Auditor\" single\n      recommend no-position\n"
        );
    }

    #[test]
    fn a_modifier_is_inherited_through_every_enclosing_section_and_ballot() {
        assert_eq!(
            formatted(
                "oval 1\nrequired ballot @b \"B\"\n  ballot @c \"C\"\n    section \"S\"\n      required measure @m \"M\"\n        recommend yes\n      optional measure @n \"N\"\n        recommend yes\n  optional ballot @d \"D\"\n    optional measure @o \"O\"\n      recommend yes\n    required measure @p \"P\"\n      recommend yes\nballot @e \"E\"\n  optional measure @q \"Q\"\n    recommend yes\n  required measure @r \"R\"\n    recommend yes\n"
            ),
            "oval 1\nrequired ballot @b \"B\"\n  ballot @c \"C\"\n    section \"S\"\n      measure @m \"M\"\n        recommend yes\n      optional measure @n \"N\"\n        recommend yes\n  optional ballot @d \"D\"\n    measure @o \"O\"\n      recommend yes\n    required measure @p \"P\"\n      recommend yes\nballot @e \"E\"\n  measure @q \"Q\"\n    recommend yes\n  required measure @r \"R\"\n    recommend yes\n"
        );
    }

    #[test]
    fn exhaustive_is_removed_from_a_retention_contest_and_extensible_is_left_for_the_author() {
        assert_eq!(
            formatted(
                "oval 1\nextensible ballot @b \"B\"\n  exhaustive contest @r \"R\" retention\n    campaign @u \"U\"\n    recommend no-position\n  extensible contest @s \"S\" retention\n    campaign @v \"V\"\n    recommend no-position\n  exhaustive contest @k \"K\" single\n    campaign @w \"W\"\n    recommend no-position\nballot @c \"C\"\n  exhaustive contest @t \"T\" retention\n    campaign @x \"X\"\n    recommend no-position\n"
            ),
            "oval 1\nextensible ballot @b \"B\"\n  contest @r \"R\" retention\n    campaign @u \"U\"\n    recommend no-position\n  extensible contest @s \"S\" retention\n    campaign @v \"V\"\n    recommend no-position\n  exhaustive contest @k \"K\" single\n    campaign @w \"W\"\n    recommend no-position\nballot @c \"C\"\n  contest @t \"T\" retention\n    campaign @x \"X\"\n    recommend no-position\n"
        );
    }
}
