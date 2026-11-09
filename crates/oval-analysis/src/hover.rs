use std::fmt::Write;

use oval_core::SPECIFICATION_URL;
use oval_core::index::{EntityKind, Index};
use oval_syntax::ast::{AstNode, Contest, ContestKind, Document};
use oval_syntax::parser::parse;
use oval_syntax::{TextRange, TextSize};
use serde::Serialize;

use crate::names::display_names;
use crate::symbols::serialize_range;

/// Text to show for the token at a position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hover {
    /// The range of the token the text describes.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// The text, as Markdown.
    pub contents: String,
}

/// Text describing the identifier or keyword at `offset` in `source`.
///
/// For an identifier, the text names what it declares, with the name or title the declaration
/// gives it and, for a candidate, its party. For a keyword, the text is one sentence on what the
/// keyword introduces and a link to its section of the specification. `None` when neither is at
/// `offset`, and for an identifier that nothing declares. An offset past the end of `source` or
/// inside a character is moved back to the nearest character boundary.
pub fn hover(source: &str, offset: TextSize) -> Option<Hover> {
    let offset = TextSize::try_from(crate::clamp_to_char_boundary(source, offset)).ok()?;
    let parsed = parse(source).ok()?;
    let document = Document::cast(parsed.syntax())?;
    let index = Index::build(&document);
    if let Some(found) = identifier(&document, &index, offset) {
        return Some(found);
    }
    let token = parsed
        .syntax()
        .token_at_offset(offset)
        .find(|token| token.kind().is_keyword())?;
    let keyword = token.kind().as_keyword()?;
    let (summary, section) = describe(keyword.text())?;
    Some(Hover {
        range: token.text_range(),
        contents: format!(
            "**keyword** `{}`\n\n{summary}\n\n[{section}]({SPECIFICATION_URL}#sec-{})",
            keyword.text(),
            section.replace(' ', "-"),
        ),
    })
}

fn identifier(document: &Document, index: &Index, offset: TextSize) -> Option<Hover> {
    let touches = |range: TextRange| range.start() <= offset && offset <= range.end();
    let (name, range) = index
        .declarations
        .iter()
        .find(|declaration| touches(declaration.range))
        .map(|declaration| (declaration.name.clone(), declaration.range))
        .or_else(|| {
            index
                .references
                .iter()
                .find(|reference| touches(reference.range))
                .map(|reference| (reference.name.clone(), reference.range))
        })?;
    let declaration = index.resolve(&name)?;
    let names = display_names(document, index);
    let mut contents = format!("**{}** `@{name}`", declaration.kind.noun());
    if let Some(display) = names.get(&name) {
        let _ = write!(contents, "\n\n{display}");
    }
    match declaration.kind {
        EntityKind::Candidate => {
            let party = document
                .candidates()
                .find(|candidate| candidate.id().is_some_and(|id| id.name() == name))
                .and_then(|candidate| candidate.party())
                .and_then(|party| party.id())
                .map(|id| id.name())
                .and_then(|party| names.get(&party).cloned());
            if let Some(party) = party {
                let _ = write!(contents, "\n\n{party}");
            }
        }
        EntityKind::Contest => {
            let method = index
                .contests
                .iter()
                .find(|contest| contest.id().is_some_and(|id| id.name() == name))
                .and_then(Contest::kind)
                .map(|kind| match kind {
                    ContestKind::Single => "Vote for one".to_owned(),
                    ContestKind::Multiple { seats } => seats
                        .map_or("Vote for several".to_owned(), |n| {
                            format!("Vote for up to {n}")
                        }),
                    ContestKind::Ranked { max } => max
                        .map_or("Rank the candidates".to_owned(), |n| {
                            format!("Rank up to {n}")
                        }),
                    ContestKind::Retention => "Yes or no".to_owned(),
                });
            if let Some(method) = method {
                let _ = write!(contents, "\n\n{method}");
            }
        }
        _ => {}
    }
    let uses = index
        .references
        .iter()
        .filter(|reference| reference.name == name)
        .count();
    let _ = write!(
        contents,
        "\n\n{}",
        match uses {
            0 => "Not referenced".to_owned(),
            1 => "Referenced once".to_owned(),
            n => format!("Referenced {n} times"),
        }
    );
    Some(Hover { range, contents })
}

#[allow(clippy::too_many_lines)]
fn describe(keyword: &str) -> Option<(&'static str, &'static str)> {
    Some(match keyword {
        "oval" => (
            "The version marker, which opens every document with the major version of the language it is written in.",
            "Document",
        ),
        "guide" => ("The name of the guide.", "Guide"),
        "election" => ("The official name of the election.", "Guide"),
        "date" => ("Election day, as `YYYY-MM-DD`.", "Guide"),
        "note" => (
            "Commentary in Markdown, under the construct it concerns.",
            "Fields and Blocks",
        ),
        "party" => (
            "A party, declared at the top level or named on a candidate or campaign.",
            "Party",
        ),
        "abbreviation" => (
            "The short form of a party's name, as printed on the ballot.",
            "Party",
        ),
        "website" => ("The official site of a party or campaign.", "Party"),
        "candidate" => (
            "A person running for office, declared at the top level or inline in a campaign.",
            "Candidate",
        ),
        "designation" => (
            "A candidate's ballot designation, such as an occupation or current office.",
            "Candidate",
        ),
        "ballot" => (
            "The questions one electorate sees, nested for narrower electorates.",
            "Ballot",
        ),
        "eligibility" => ("Who votes on a ballot, in prose.", "Ballot"),
        "section" => ("A titled group of questions within a ballot.", "Section"),
        "contest" => (
            "An office chosen among campaigns, with its voting method.",
            "Contest",
        ),
        "single" => (
            "A contest in which the voter chooses one campaign.",
            "Contest",
        ),
        "multiple" => (
            "A contest in which the voter chooses up to a number of campaigns.",
            "Contest",
        ),
        "ranked" => ("A contest in which the voter ranks campaigns.", "Contest"),
        "max" => ("The most campaigns a voter may rank.", "Contest"),
        "retention" => (
            "A yes-or-no contest on keeping one officeholder.",
            "Contest",
        ),
        "subtitle" => (
            "A second line of the title, as printed on the ballot.",
            "Contest",
        ),
        "tabulation" => (
            "How votes are counted, or what it takes to pass, in prose.",
            "Contest",
        ),
        "campaign" => (
            "One or more candidates running together in a contest.",
            "Campaign",
        ),
        "incumbent" => ("The campaign currently occupies the office.", "Campaign"),
        "withdrawn" => (
            "The campaign has ended but remains on the ballot.",
            "Campaign",
        ),
        "recommend" => (
            "The author's recommendation for a contest or measure.",
            "Contest Recommendation",
        ),
        "support" => (
            "Campaigns the author supports, named by candidate, in tiers of preference.",
            "Contest Recommendation",
        ),
        "oppose" => (
            "Campaigns the author advises against, named by candidate.",
            "Contest Recommendation",
        ),
        "justification" => (
            "Why the author recommends this, in Markdown.",
            "Contest Recommendation",
        ),
        "no-position" => (
            "The author deliberately takes no position.",
            "Contest Recommendation",
        ),
        "measure" => ("A yes-or-no question on the ballot.", "Measure"),
        "description" => (
            "The official ballot text of a measure, copied verbatim.",
            "Measure",
        ),
        "if_yes" => ("The official text of what a yes vote does.", "Measure"),
        "if_no" => ("The official text of what a no vote does.", "Measure"),
        "placement" => (
            "How the measure reached the ballot, such as a legislative referral.",
            "Measure",
        ),
        "fiscal_impact" => ("The official fiscal estimate, in prose.", "Measure"),
        "supporters" => ("Who supports the measure, or `none`.", "Measure"),
        "opponents" => ("Who opposes the measure, or `none`.", "Measure"),
        "none" => ("Researched, and none known.", "Measure"),
        "yes" => ("A recommendation to vote yes.", "Measure Recommendation"),
        "no" => ("A recommendation to vote no.", "Measure Recommendation"),
        _ => return None,
    })
}
