use oval_syntax::ast::{self, AstNode, Identifier};
use oval_syntax::{SyntaxKind, SyntaxNode};

pub(crate) fn head(node: &SyntaxNode) -> Option<SyntaxNode> {
    let reference = Identifier::cast(node.clone())?;
    let line = node.parent()?;
    if !matches!(line.kind(), SyntaxKind::Support | SyntaxKind::Oppose) {
        return None;
    }
    let name = name(&reference)?;
    let contest = line.ancestors().find_map(ast::Contest::cast)?;
    contest.campaigns().find_map(|campaign| {
        let members: Vec<Identifier> = campaign
            .members()
            .filter_map(|member| member.identifier())
            .chain(
                campaign
                    .party_members()
                    .filter_map(|member| member.identifier()),
            )
            .collect();
        if members
            .iter()
            .any(|member| self::name(member).as_ref() == Some(&name))
        {
            members.first().map(|head| head.syntax().clone())
        } else {
            None
        }
    })
}

fn name(identifier: &Identifier) -> Option<String> {
    Some(identifier.name()?.text().to_owned())
}

#[cfg(test)]
mod tests {
    use crate::format;

    fn formatted(text: &str) -> String {
        let formatted = format(text).unwrap();
        assert_eq!(format(&formatted).unwrap(), formatted, "formats to itself");
        formatted
    }

    fn contest(campaigns: &str, recommendation: &str) -> String {
        formatted(&format!(
            "oval 1\nballot @b \"B\"\n  contest @k \"K\" ranked\n{campaigns}    recommend\n{recommendation}"
        ))
    }

    fn recommendation(formatted: &str) -> &str {
        &formatted[formatted.find("    recommend\n").unwrap() + "    recommend\n".len()..]
    }

    #[test]
    fn a_reference_to_any_member_is_rewritten_to_name_the_head() {
        let formatted = contest(
            "    campaign\n      @jlee \"Jordan Lee\" as \"Governor\"\n      @mpatel \"Maya Patel\" as \"Lieutenant Governor\"\n    campaign party\n      @dem \"Democratic\"\n      @lab \"Labour\"\n    campaign @cfox \"Carl Fox\"\n",
            "      support @mpatel\n      support @lab @jlee\n      oppose @cfox\n",
        );
        assert_eq!(
            recommendation(&formatted),
            "      support @jlee\n      support @dem @jlee\n      oppose @cfox\n"
        );
    }

    #[test]
    fn a_member_of_two_campaigns_resolves_to_the_first() {
        let formatted = contest(
            "    campaign\n      @a \"A\"\n      @c \"C\"\n    campaign\n      @d \"D\"\n      @c\n",
            "      oppose @c\n",
        );
        assert_eq!(recommendation(&formatted), "      oppose @a\n");
        assert!(
            formatted.contains("      @d \"D\"\n      @c\n"),
            "{formatted}"
        );
    }

    #[test]
    fn a_reference_that_resolves_to_no_campaign_and_a_write_in_are_left_as_written() {
        let formatted = contest(
            "    campaign\n      @a \"A\"\n        party @dem \"Democratic\"\n      @c \"C\"\n",
            "      support @nobody write-in \"Jane Doe\"\n      oppose @dem write-in\n",
        );
        assert_eq!(
            recommendation(&formatted),
            "      support @nobody write-in \"Jane Doe\"\n      oppose @dem write-in\n"
        );
    }
}
