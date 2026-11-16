use oval_core::{Declaration, Entity};
use oval_syntax::ast::{self, AstNode, Campaign, Member, PartyRef};
use oval_syntax::{Keyword, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize};

use crate::Analysis;

/// Text describing what's under the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// The identifier or keyword the text describes.
    pub range: TextRange,
    /// The description in Markdown.
    pub contents: String,
}

impl Analysis {
    /// Describes the identifier or keyword at `offset`, or returns `None` when there's neither.
    ///
    /// An identifier is described by what it declares, with the declaration's name and, for a
    /// candidate, the name of their party. A name nothing declares has no description. A keyword is
    /// described in one sentence on what it introduces.
    pub fn hover(&self, offset: TextSize) -> Option<Hover> {
        if let Some(symbol) = self.symbol_at(offset) {
            let declaration = self.definition(offset)?;
            return Some(Hover {
                range: symbol.identifier().syntax().text_range(),
                contents: self.describe(declaration),
            });
        }
        let token = self
            .report()
            .parse()
            .syntax()
            .token_at_offset(offset)
            .find(|token| keyword(token).is_some())?;
        let keyword = keyword(&token)?;
        Some(Hover {
            range: token.text_range(),
            contents: format!("`{}` {}.", keyword.text(), sentence(keyword)),
        })
    }

    fn describe(&self, declaration: &Declaration) -> String {
        let kind = match declaration.entity {
            Entity::Party => "party",
            Entity::Candidate => "candidate",
            Entity::Ballot => "ballot",
            Entity::Contest => "contest",
            Entity::Measure => "measure",
        };
        let name = match declaration.name() {
            Some(name) => format!("**{}**", name.value()),
            None => format!("`{}`", declaration.identifier.syntax().text()),
        };
        let party = declaration
            .identifier
            .syntax()
            .parent()
            .filter(|_| declaration.entity == Entity::Candidate)
            .and_then(|node| party_field(&node))
            .and_then(|party| self.party_name(&party));
        match party {
            Some(party) => format!("{kind} {name}, {party}"),
            None => format!("{kind} {name}"),
        }
    }

    fn party_name(&self, party: &PartyRef) -> Option<String> {
        let symbols = self.symbols();
        let declaration = symbols
            .declarations()
            .get(symbols.resolve(&party.identifier()?)?)?;
        (declaration.entity == Entity::Party)
            .then(|| declaration.name())
            .flatten()
            .map(|name| name.value().into_owned())
    }
}

fn party_field(declaration: &SyntaxNode) -> Option<PartyRef> {
    if let Some(candidate) = ast::Candidate::cast(declaration.clone()) {
        return candidate.party();
    }
    let member = Member::cast(declaration.clone())?;
    let campaign = member.syntax().ancestors().find_map(Campaign::cast)?;
    if campaign.is_block_form() {
        member.party()
    } else {
        campaign.party()
    }
}

fn keyword(token: &SyntaxToken) -> Option<Keyword> {
    let names_an_identifier = token
        .prev_token()
        .is_some_and(|previous| previous.kind() == SyntaxKind::AtSign);
    if token.kind() != SyntaxKind::Word || names_an_identifier {
        return None;
    }
    Keyword::from_text(token.text())
}

fn sentence(keyword: Keyword) -> &'static str {
    match keyword {
        Keyword::Oval => "gives the major version of Oval the document is written in",
        Keyword::Guide => "gives the voter guide's name",
        Keyword::Election => "names the election the guide covers",
        Keyword::Date => "gives the date of the election",
        Keyword::Note => "adds the author's note in Markdown",
        Keyword::Party => "declares or names a political party",
        Keyword::Candidate => "declares a candidate",
        Keyword::Ballot => "declares a ballot, or a sub-ballot for a narrower electorate",
        Keyword::Section => "groups a ballot's contests and measures under a title",
        Keyword::Contest => "declares a race between campaigns for an office",
        Keyword::Campaign => "adds candidates or parties running together in a contest",
        Keyword::Measure => "declares a question that voters answer yes or no",
        Keyword::Abbreviation => "gives the short form printed beside a party's candidates",
        Keyword::Website => "gives a website",
        Keyword::Designation => "gives the occupation or title printed beside a name",
        Keyword::Eligibility => "says who votes on the ballot",
        Keyword::Single => "makes the contest a vote for one choice",
        Keyword::Multiple => "makes the contest a vote for up to a number of choices",
        Keyword::Ranked => "makes the contest a ranking of choices by preference",
        Keyword::Retention => "makes the contest a yes or no on keeping an officeholder",
        Keyword::Seats => "gives how many candidates the contest elects",
        Keyword::Max => "gives how many choices a voter may rank",
        Keyword::Min => "gives how many choices a ballot that marks the contest must mark",
        Keyword::All => "requires every printed choice to be ranked",
        Keyword::Subtitle => "gives a line printed under the title",
        Keyword::Tabulation => "says how the votes are counted",
        Keyword::Incumbent => "marks the campaign that holds the office now",
        Keyword::Withdrawn => "marks a campaign that ended but is still on the ballot",
        Keyword::As => "gives a member's role on the ticket",
        Keyword::Required => "means the voter must answer the question",
        Keyword::Optional => "means the voter may leave the question blank",
        Keyword::Extensible => "lets the voter write in a choice",
        Keyword::Exhaustive => "allows only the choices printed on the ballot",
        Keyword::Recommend => "gives the author's recommendation",
        Keyword::Support => "lists one tier of choices the author recommends",
        Keyword::Oppose => "lists the choices the author advises against",
        Keyword::Justification => "gives the author's reasoning in Markdown",
        Keyword::Yes => "recommends voting yes",
        Keyword::No => "recommends voting no",
        Keyword::NoPosition => "says the author takes no position",
        Keyword::Abstain => "advises leaving the question blank",
        Keyword::WriteIn => "is a choice the voter writes on the ballot by hand",
        Keyword::Description => "gives the measure's official ballot text in Markdown",
        Keyword::IfYes => "says what happens if the measure passes, in Markdown",
        Keyword::IfNo => "says what happens if the measure fails, in Markdown",
        Keyword::Placement => "says how the measure got onto the ballot",
        Keyword::FiscalImpact => "summarizes what the measure costs",
        Keyword::Supporters => "lists who supports the measure",
        Keyword::Opponents => "lists who opposes the measure",
        Keyword::NoneValue => "says research found no one",
    }
}

#[cfg(test)]
mod tests {
    use oval_syntax::{TextRange, TextSize};

    use crate::Analysis;

    const SOURCE: &str = "oval 1\nparty @dem \"Democratic Party\"\ncandidate @n \"Nadia Reyes\"\n  party @dem\ncandidate @x \"Xavier\"\nballot @b \"City\"\n  contest @k \"Mayor\" single\n    campaign @n\n    campaign @c \"Carl Fox\"\n      party @grn \"Green Party\"\n    campaign\n      @j \"Jordan Lee\"\n        party @dem\n      @x\n    recommend\n      support @n\n      oppose @nobody write-in\n  measure @m \"Measure M\"\n    recommend no-position\n";

    fn at(needle: &str, nth: usize) -> TextSize {
        let offset = SOURCE.match_indices(needle).nth(nth).unwrap().0;
        TextSize::try_from(offset).unwrap()
    }

    fn contents(offset: TextSize) -> Option<String> {
        Analysis::new(SOURCE)
            .unwrap()
            .hover(offset)
            .map(|hover| hover.contents)
    }

    #[test]
    fn an_identifier_is_described_by_what_it_declares() {
        assert_eq!(
            contents(at("@dem", 0)).as_deref(),
            Some("party **Democratic Party**")
        );
        assert_eq!(contents(at("@b", 0)).as_deref(), Some("ballot **City**"));
        assert_eq!(contents(at("@k", 0)).as_deref(), Some("contest **Mayor**"));
        assert_eq!(
            contents(at("@m", 0)).as_deref(),
            Some("measure **Measure M**")
        );
        assert_eq!(
            contents(at("@x", 0)).as_deref(),
            Some("candidate **Xavier**")
        );
    }

    #[test]
    fn a_candidate_is_described_with_their_party_however_it_is_declared() {
        assert_eq!(
            contents(at("@n", 0)).as_deref(),
            Some("candidate **Nadia Reyes**, Democratic Party")
        );
        assert_eq!(
            contents(at("@c", 0)).as_deref(),
            Some("candidate **Carl Fox**, Green Party")
        );
        assert_eq!(
            contents(at("@j", 0)).as_deref(),
            Some("candidate **Jordan Lee**, Democratic Party")
        );
    }

    #[test]
    fn a_reference_is_described_by_its_declaration_and_highlights_itself() {
        let analysis = Analysis::new(SOURCE).unwrap();
        let reference = at("@n", 2);
        let hover = analysis.hover(reference).unwrap();
        assert_eq!(
            hover.contents,
            "candidate **Nadia Reyes**, Democratic Party"
        );
        assert_eq!(hover.range, TextRange::at(reference, TextSize::of("@n")));
        assert_eq!(contents(at("@nobody", 0)), None);
    }

    #[test]
    fn a_keyword_is_described_in_a_sentence() {
        assert_eq!(
            contents(at("ballot", 0)).as_deref(),
            Some("`ballot` declares a ballot, or a sub-ballot for a narrower electorate.")
        );
        assert_eq!(
            contents(at("no-position", 0) + TextSize::from(3)).as_deref(),
            Some("`no-position` says the author takes no position.")
        );
        assert_eq!(
            contents(at("write-in", 0)).as_deref(),
            Some("`write-in` is a choice the voter writes on the ballot by hand.")
        );
        let analysis = Analysis::new(SOURCE).unwrap();
        let hover = analysis.hover(at("single", 0)).unwrap();
        assert_eq!(
            hover.range,
            TextRange::at(at("single", 0), TextSize::of("single"))
        );
    }

    #[test]
    fn a_keyword_written_as_an_identifier_is_not_described_as_a_keyword() {
        let source = "oval 1\nballot @b \"B\"\n  abbreviation @single\n";
        let offset = TextSize::try_from(source.find("single").unwrap()).unwrap();
        assert_eq!(Analysis::new(source).unwrap().hover(offset), None);
    }

    #[test]
    fn a_party_field_that_names_a_candidate_gives_no_party() {
        let source = "oval 1\ncandidate @a \"A\"\ncandidate @c \"C\"\n  party @a\n";
        let offset = TextSize::try_from(source.find("@c").unwrap()).unwrap();
        let hover = Analysis::new(source).unwrap().hover(offset).unwrap();
        assert_eq!(hover.contents, "candidate **C**");
    }

    #[test]
    fn text_that_is_neither_an_identifier_nor_a_keyword_has_no_description() {
        assert_eq!(contents(at("Democratic Party", 0)), None);
        assert_eq!(contents(at("1\n", 0)), None);
    }
}
