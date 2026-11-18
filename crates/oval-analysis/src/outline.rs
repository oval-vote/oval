use oval_syntax::ast::{
    self, AstNode, AstToken, BallotItem, Identifier, SectionItem, StringLiteral,
};
use oval_syntax::{SyntaxElement, SyntaxNode, SyntaxToken, TextRange, TokenKind};

use crate::Analysis;

/// The kind of construct an [`OutlineItem`] stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutlineKind {
    /// A root ballot or a sub-ballot.
    Ballot,
    /// A section of a ballot.
    Section,
    /// A contest.
    Contest,
    /// A measure.
    Measure,
}

/// One construct in a document's outline, with the constructs nested in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineItem {
    /// What the construct is.
    pub kind: OutlineKind,
    /// The construct's identifier without its `@`, or `None` for a section or when it's missing.
    pub id: Option<String>,
    /// The construct's name or title, or `None` when it's missing.
    pub title: Option<String>,
    /// The whole construct, from its first token through its block.
    pub range: TextRange,
    /// The part to highlight when the item is chosen, which is the identifier, a section's title,
    /// or the construct's first token when neither is written.
    pub selection: TextRange,
    /// The constructs inside this one, in ballot order.
    pub children: Vec<OutlineItem>,
}

impl Analysis {
    /// Returns the document's ballots as a tree of ballots, sections, contests, and measures, with
    /// the root ballots in document order.
    ///
    /// Parties and candidates aren't in the outline, since they're declarations rather than places
    /// on a ballot. [`symbols`](Self::symbols) lists them.
    pub fn outline(&self) -> Vec<OutlineItem> {
        self.report()
            .parse()
            .document()
            .ballots()
            .map(|ballot| self::ballot(&ballot))
            .collect()
    }
}

fn ballot(ballot: &ast::Ballot) -> OutlineItem {
    let children = ballot
        .items()
        .map(|item| match item {
            BallotItem::Section(section) => self::section(&section),
            BallotItem::Contest(contest) => self::contest(&contest),
            BallotItem::Measure(measure) => self::measure(&measure),
            BallotItem::Ballot(sub_ballot) => self::ballot(&sub_ballot),
        })
        .collect();
    item(
        OutlineKind::Ballot,
        ballot.syntax(),
        ballot.identifier(),
        ballot.name(),
        children,
    )
}

fn section(section: &ast::Section) -> OutlineItem {
    let children = section
        .items()
        .map(|item| match item {
            SectionItem::Contest(contest) => self::contest(&contest),
            SectionItem::Measure(measure) => self::measure(&measure),
        })
        .collect();
    item(
        OutlineKind::Section,
        section.syntax(),
        None,
        section.title(),
        children,
    )
}

fn contest(contest: &ast::Contest) -> OutlineItem {
    item(
        OutlineKind::Contest,
        contest.syntax(),
        contest.identifier(),
        contest.name(),
        Vec::new(),
    )
}

fn measure(measure: &ast::Measure) -> OutlineItem {
    item(
        OutlineKind::Measure,
        measure.syntax(),
        measure.identifier(),
        measure.name(),
        Vec::new(),
    )
}

fn item(
    kind: OutlineKind,
    node: &SyntaxNode,
    identifier: Option<Identifier>,
    title: Option<StringLiteral>,
    children: Vec<OutlineItem>,
) -> OutlineItem {
    let range = content_range(node);
    let selection = identifier
        .as_ref()
        .map(|identifier| identifier.syntax().text_range())
        .or_else(|| title.as_ref().map(|title| title.syntax().text_range()))
        .or_else(|| node.first_token().map(|token| token.text_range()))
        .unwrap_or(range);
    OutlineItem {
        kind,
        id: identifier
            .and_then(|identifier| identifier.name())
            .map(|name| name.text().to_owned()),
        title: title.map(|title| title.value().into_owned()),
        range,
        selection,
        children,
    }
}

fn content_range(node: &SyntaxNode) -> TextRange {
    let mut tokens = node
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(is_content);
    let Some(first) = tokens.next() else {
        return node.text_range();
    };
    let last = tokens.last().unwrap_or_else(|| first.clone());
    first.text_range().cover(last.text_range())
}

fn is_content(token: &SyntaxToken) -> bool {
    !token.text_range().is_empty() && !token.kind().token_kind().is_some_and(TokenKind::is_ignored)
}

#[cfg(test)]
mod tests {
    use oval_syntax::{TextRange, TextSize};

    use super::{OutlineItem, OutlineKind};
    use crate::Analysis;

    const SOURCE: &str = "oval 1\nparty @dem \"Democratic\"\nballot @state \"State\"\n  section \"Offices\"\n    contest @gov \"Governor\" single\n      campaign @a \"A\"\n      recommend no-position\n  ballot @city \"City\"\n    measure @m \"Measure M\"\n      recommend yes\nballot @county \"County\"\n  measure @n \"Measure N\"\n    recommend no\n";

    fn outline(item: &OutlineItem) -> String {
        let name = item
            .id
            .clone()
            .or_else(|| item.title.clone())
            .unwrap_or_default();
        let kind = format!("{:?}", item.kind).to_lowercase();
        if item.children.is_empty() {
            format!("{kind} {name}")
        } else {
            let children: Vec<String> = item.children.iter().map(outline).collect();
            format!("{kind} {name} [{}]", children.join(", "))
        }
    }

    fn range(source: &str, start: &str, end: &str) -> TextRange {
        let start = source.find(start).unwrap();
        let end = source.find(end).unwrap() + end.len();
        TextRange::new(
            TextSize::try_from(start).unwrap(),
            TextSize::try_from(end).unwrap(),
        )
    }

    #[test]
    fn the_outline_is_the_ballot_tree_without_declarations() {
        let outline: Vec<String> = Analysis::new(SOURCE)
            .unwrap()
            .outline()
            .iter()
            .map(outline)
            .collect();
        assert_eq!(
            outline,
            [
                "ballot state [section Offices [contest gov], ballot city [measure m]]",
                "ballot county [measure n]",
            ]
        );
    }

    #[test]
    fn an_item_has_its_identifier_title_and_ranges() {
        let outline = Analysis::new(SOURCE).unwrap().outline();
        let state = &outline[0];
        assert_eq!(state.kind, OutlineKind::Ballot);
        assert_eq!(state.id.as_deref(), Some("state"));
        assert_eq!(state.title.as_deref(), Some("State"));
        assert_eq!(
            state.range,
            range(SOURCE, "ballot @state", "recommend yes\n")
        );
        assert_eq!(state.selection, range(SOURCE, "@state", "@state"));
        let section = &state.children[0];
        assert_eq!(section.id, None);
        assert_eq!(section.title.as_deref(), Some("Offices"));
        assert_eq!(
            section.selection,
            range(SOURCE, "\"Offices\"", "\"Offices\"")
        );
        let governor = &section.children[0];
        assert_eq!(governor.title.as_deref(), Some("Governor"));
        assert_eq!(
            governor.range,
            range(SOURCE, "contest @gov", "recommend no-position\n")
        );
        assert_eq!(governor.selection, range(SOURCE, "@gov", "@gov"));
    }

    #[test]
    fn a_range_leaves_out_comments_and_indentation_after_the_construct() {
        let source = "oval 1\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n    # about the next one\n\n  measure @n \"N\"\n    recommend no\n";
        let outline = Analysis::new(source).unwrap().outline();
        assert_eq!(
            outline[0].children[0].range,
            range(source, "measure @m", "recommend yes\n")
        );
    }

    #[test]
    fn an_item_missing_its_identifier_and_title_selects_its_first_token() {
        let source = "oval 1\nrequired ballot\n  measure @m \"M\"\n    recommend yes\n";
        let outline = Analysis::new(source).unwrap().outline();
        assert_eq!(outline[0].id, None);
        assert_eq!(outline[0].title, None);
        assert_eq!(outline[0].selection, range(source, "required", "required"));
        assert_eq!(outline[0].children[0].id.as_deref(), Some("m"));
    }
}
