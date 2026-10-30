//! Typed views over the syntax tree. Every accessor is total: a malformed
//! tree yields `None` or an empty list, never a panic.

use crate::kind::SyntaxKind::{self, *};
use crate::{Segment, SyntaxNode, SyntaxToken, TextRange};

macro_rules! node {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name(SyntaxNode);
        impl $name {
            pub fn cast(node: SyntaxNode) -> Option<Self> {
                (node.kind() == $kind).then_some(Self(node))
            }
            pub fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
            pub fn range(&self) -> TextRange {
                self.0.text_range()
            }
        }
    };
}

node!(Root, ROOT);
node!(Magic, MAGIC);
node!(Header, HEADER);
node!(Section, SECTION);
node!(Heading, HEADING);
node!(Description, DESCRIPTION);
node!(KindLine, KIND_LINE);
node!(Field, FIELD);
node!(Candidate, CANDIDATE);
node!(Paragraph, PARAGRAPH);

fn child<T>(node: &SyntaxNode, cast: fn(SyntaxNode) -> Option<T>) -> Option<T> {
    node.children().find_map(cast)
}

fn children<T: 'static>(
    node: &SyntaxNode,
    cast: fn(SyntaxNode) -> Option<T>,
) -> impl Iterator<Item = T> + '_ {
    node.children().filter_map(cast)
}

fn token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == kind)
}

fn text_of(node: &SyntaxNode, kind: SyntaxKind) -> Option<Segment> {
    let n = node.children().find(|c| c.kind() == kind)?;
    let t = token(&n, TEXT)?;
    Some(Segment {
        text: t.text().to_string(),
        range: t.text_range(),
    })
}

/// The TEXT of every CONTINUATION child, in order.
fn continuations(node: &SyntaxNode) -> Vec<Segment> {
    node.children()
        .filter(|c| c.kind() == CONTINUATION)
        .filter_map(|c| token(&c, TEXT))
        .map(|t| Segment {
            text: t.text().to_string(),
            range: t.text_range(),
        })
        .collect()
}

impl Root {
    pub fn magic(&self) -> Option<Magic> {
        child(&self.0, Magic::cast)
    }
    pub fn header(&self) -> Option<Header> {
        child(&self.0, Header::cast)
    }
    pub fn sections(&self) -> impl Iterator<Item = Section> + '_ {
        children(&self.0, Section::cast)
    }
}

impl Magic {
    pub fn version(&self) -> Option<u32> {
        token(&self.0, NUMBER)?.text().parse().ok()
    }
}

impl Header {
    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        children(&self.0, Field::cast)
    }
}

/// What a section's body contains, in source order. `Error` items are
/// misplaced constructs the parser kept but the model ignores.
#[derive(Clone, Debug)]
pub enum Item {
    Field(Field),
    Candidate(Candidate),
    Paragraph(Paragraph),
    Section(Section),
    Error(SyntaxNode),
}

impl Section {
    pub fn heading(&self) -> Option<Heading> {
        child(&self.0, Heading::cast)
    }
    pub fn description(&self) -> Option<Description> {
        child(&self.0, Description::cast)
    }
    pub fn kind_line(&self) -> Option<KindLine> {
        child(&self.0, KindLine::cast)
    }
    /// The kind keyword, or `None` for a plain container section.
    pub fn kind(&self) -> Option<String> {
        self.kind_line()?.keyword()
    }
    pub fn items(&self) -> impl Iterator<Item = Item> + '_ {
        self.0.children().filter_map(|c| match c.kind() {
            FIELD => Field::cast(c).map(Item::Field),
            CANDIDATE => Candidate::cast(c).map(Item::Candidate),
            PARAGRAPH => Paragraph::cast(c).map(Item::Paragraph),
            SECTION => Section::cast(c).map(Item::Section),
            ERROR => Some(Item::Error(c)),
            _ => None,
        })
    }
    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        children(&self.0, Field::cast)
    }
    pub fn candidates(&self) -> impl Iterator<Item = Candidate> + '_ {
        children(&self.0, Candidate::cast)
    }
    pub fn paragraphs(&self) -> impl Iterator<Item = Paragraph> + '_ {
        children(&self.0, Paragraph::cast)
    }
    pub fn sections(&self) -> impl Iterator<Item = Section> + '_ {
        children(&self.0, Section::cast)
    }
}

impl Heading {
    pub fn level(&self) -> usize {
        token(&self.0, HASHES).map_or(0, |t| t.text().len())
    }
    pub fn name(&self) -> Option<Segment> {
        text_of(&self.0, NAME)
    }
    pub fn id(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }
}

impl Description {
    pub fn segments(&self) -> Vec<Segment> {
        self.0
            .children()
            .filter(|c| c.kind() == DESC_LINE)
            .filter_map(|c| token(&c, TEXT))
            .map(|t| Segment {
                text: t.text().to_string(),
                range: t.text_range(),
            })
            .collect()
    }
}

impl KindLine {
    pub fn keyword(&self) -> Option<String> {
        Some(token(&self.0, KEYWORD)?.text().to_string())
    }
    pub fn tags(&self) -> Vec<Segment> {
        self.0
            .children()
            .filter(|c| c.kind() == TAG)
            .filter_map(|c| token(&c, TEXT))
            .map(|t| Segment {
                text: t.text().to_string(),
                range: t.text_range(),
            })
            .collect()
    }
}

impl Field {
    pub fn key(&self) -> Option<Segment> {
        text_of(&self.0, KEY)
    }
    /// The value's first line and continuation lines, in order. Empty when
    /// the field has no value.
    pub fn segments(&self) -> Vec<Segment> {
        let mut v: Vec<Segment> = text_of(&self.0, VALUE).into_iter().collect();
        v.extend(continuations(&self.0));
        v
    }
    pub fn value(&self) -> String {
        crate::join_segments(&self.segments())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marker {
    Pick,
    Avoid,
    Neutral,
    Rank(u32),
}

impl Candidate {
    pub fn marker(&self) -> Option<Marker> {
        let m = self.0.children().find(|c| c.kind() == MARKER)?;
        let t = m
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .next()?;
        Some(match t.kind() {
            PLUS => Marker::Pick,
            MINUS => Marker::Avoid,
            STAR => Marker::Neutral,
            NUMBER => Marker::Rank(t.text().parse().unwrap_or(u32::MAX)),
            _ => return None,
        })
    }
    pub fn marker_range(&self) -> TextRange {
        self.0
            .children()
            .find(|c| c.kind() == MARKER)
            .map_or(self.range(), |m| m.text_range())
    }
    pub fn name(&self) -> Option<Segment> {
        text_of(&self.0, NAME)
    }
    pub fn labels(&self) -> Vec<Segment> {
        self.0
            .children()
            .filter(|c| c.kind() == LABELS)
            .flat_map(|l| l.children().collect::<Vec<_>>())
            .filter(|c| c.kind() == LABEL)
            .filter_map(|c| token(&c, TEXT))
            .map(|t| Segment {
                text: t.text().to_string(),
                range: t.text_range(),
            })
            .collect()
    }
    pub fn designation(&self) -> Option<Segment> {
        text_of(&self.0, DESIGNATION)
    }
    pub fn id(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }
    pub fn note(&self) -> Vec<Segment> {
        continuations(&self.0)
    }
}

impl Paragraph {
    pub fn segments(&self) -> Vec<Segment> {
        self.0
            .children()
            .filter(|c| c.kind() == PROSE_LINE)
            .filter_map(|c| token(&c, TEXT))
            .map(|t| Segment {
                text: t.text().to_string(),
                range: t.text_range(),
            })
            .collect()
    }
}
