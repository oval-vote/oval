use oval_core::index::{EntityKind, Index};
use oval_syntax::ast::Document;
use oval_syntax::{TextRange, TextSize};
use serde::Serialize;

/// The kind of construct an identifier names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    /// A party.
    Party,
    /// A candidate.
    Candidate,
    /// A ballot, root or nested.
    Ballot,
    /// A contest.
    Contest,
    /// A measure.
    Measure,
}

impl From<EntityKind> for SymbolKind {
    fn from(kind: EntityKind) -> Self {
        match kind {
            EntityKind::Party => Self::Party,
            EntityKind::Candidate => Self::Candidate,
            EntityKind::Ballot => Self::Ballot,
            EntityKind::Contest => Self::Contest,
            EntityKind::Measure => Self::Measure,
        }
    }
}

/// A declared identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Declaration {
    /// The name without its `@`.
    pub name: String,
    /// The kind of construct declared.
    pub kind: SymbolKind,
    /// The range of the identifier, `@` included.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// The range of the name alone, which a rename replaces.
    #[serde(serialize_with = "serialize_range")]
    pub name_range: TextRange,
}

/// A use of an identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    /// The name without its `@`.
    pub name: String,
    /// The range of the identifier, `@` included.
    #[serde(serialize_with = "serialize_range")]
    pub range: TextRange,
    /// The range of the name alone, which a rename replaces.
    #[serde(serialize_with = "serialize_range")]
    pub name_range: TextRange,
    /// The position in [`Symbols::declarations`] of the declaration this reference names, or
    /// `None` when nothing declares the name.
    pub declaration: Option<usize>,
}

/// An identifier at a position, as either the declaration or a reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Symbol {
    /// The position in [`Symbols::declarations`].
    Declaration(usize),
    /// The position in [`Symbols::references`].
    Reference(usize),
}

/// Every declared identifier and every use of one in a document.
///
/// Declarations and references are in document order. A reference names the first declaration
/// of its name, as validation resolves it, so a document with a duplicate declaration still has
/// one answer for each reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Symbols {
    /// Every declared identifier.
    pub declarations: Vec<Declaration>,
    /// Every use of an identifier.
    pub references: Vec<Reference>,
}

impl Symbols {
    /// Collects the symbols of `document`.
    pub fn new(document: &Document) -> Self {
        let index = Index::build(document);
        let declarations = index
            .declarations
            .iter()
            .map(|declaration| Declaration {
                name: declaration.name.clone(),
                kind: declaration.kind.into(),
                range: declaration.range,
                name_range: declaration.name_range,
            })
            .collect();
        let references = index
            .references
            .iter()
            .map(|reference| Reference {
                name: reference.name.clone(),
                range: reference.range,
                name_range: reference.name_range,
                declaration: index.by_name.get(&reference.name).copied(),
            })
            .collect();
        Self {
            declarations,
            references,
        }
    }

    /// The identifier at `offset`, which may be at either end of it.
    ///
    /// An editor passes the cursor position. An offset just after an identifier counts, as a
    /// cursor placed at the end of a word does.
    pub fn at(&self, offset: TextSize) -> Option<Symbol> {
        let touches = |range: TextRange| range.start() <= offset && offset <= range.end();
        self.declarations
            .iter()
            .position(|declaration| touches(declaration.range))
            .map(Symbol::Declaration)
            .or_else(|| {
                self.references
                    .iter()
                    .position(|reference| touches(reference.range))
                    .map(Symbol::Reference)
            })
    }

    /// The declaration of the identifier at `offset`.
    ///
    /// `None` when no identifier is at `offset` or when the one there names nothing declared.
    pub fn definition(&self, offset: TextSize) -> Option<&Declaration> {
        match self.at(offset)? {
            Symbol::Declaration(index) => self.declarations.get(index),
            Symbol::Reference(index) => self.references[index]
                .declaration
                .and_then(|declaration| self.declarations.get(declaration)),
        }
    }

    /// Every occurrence of the identifier at `offset`, declarations and references alike, in
    /// document order.
    ///
    /// Empty when no identifier is at `offset`. The ranges include the `@`; a rename uses
    /// [`occurrence_names`](Self::occurrence_names) instead.
    pub fn occurrences(&self, offset: TextSize) -> Vec<TextRange> {
        self.occurrence_ranges(offset, |range, _| range)
    }

    /// The name ranges of every occurrence of the identifier at `offset`, in document order.
    ///
    /// These are the ranges a rename replaces. Empty when no identifier is at `offset`.
    pub fn occurrence_names(&self, offset: TextSize) -> Vec<TextRange> {
        self.occurrence_ranges(offset, |_, name_range| name_range)
    }

    fn occurrence_ranges(
        &self,
        offset: TextSize,
        pick: impl Fn(TextRange, TextRange) -> TextRange,
    ) -> Vec<TextRange> {
        let name = match self.at(offset) {
            Some(Symbol::Declaration(index)) => &self.declarations[index].name,
            Some(Symbol::Reference(index)) => &self.references[index].name,
            None => return Vec::new(),
        };
        let mut ranges: Vec<TextRange> = self
            .declarations
            .iter()
            .filter(|declaration| &declaration.name == name)
            .map(|declaration| pick(declaration.range, declaration.name_range))
            .chain(
                self.references
                    .iter()
                    .filter(|reference| &reference.name == name)
                    .map(|reference| pick(reference.range, reference.name_range)),
            )
            .collect();
        ranges.sort_by_key(|range| range.start());
        ranges
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn serialize_range<S: serde::Serializer>(
    range: &TextRange,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Range {
        start: u32,
        end: u32,
    }
    Range {
        start: range.start().into(),
        end: range.end().into(),
    }
    .serialize(serializer)
}
