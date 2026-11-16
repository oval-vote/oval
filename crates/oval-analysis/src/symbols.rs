use oval_core::{Declaration, Reference, ReferenceKind, Symbols};
use oval_syntax::ast::{AstNode, Contest, Identifier};
use oval_syntax::{TextRange, TextSize};

use crate::Analysis;

/// An identifier in the document, as a declaration or as a reference to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol<'a> {
    /// The identifier declares something.
    Declaration(&'a Declaration),
    /// The identifier refers to a declaration, which may not exist.
    Reference(&'a Reference),
}

impl Symbol<'_> {
    /// Returns the identifier as written.
    pub fn identifier(&self) -> &Identifier {
        match self {
            Self::Declaration(declaration) => &declaration.identifier,
            Self::Reference(reference) => &reference.identifier,
        }
    }
}

/// One place an identifier's name is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occurrence {
    /// The identifier, its `@` included.
    pub range: TextRange,
    /// The name alone, without its `@`, which is the text a rename replaces.
    pub name: TextRange,
    /// Whether this occurrence declares the name rather than referring to it.
    pub is_declaration: bool,
}

impl Analysis {
    /// Returns the identifier at `offset`, or `None` if there is none.
    ///
    /// An offset just past an identifier's end still counts, so a cursor right after `@dem` finds
    /// it.
    pub fn symbol_at(&self, offset: TextSize) -> Option<Symbol<'_>> {
        let contains =
            |identifier: &Identifier| identifier.syntax().text_range().contains_inclusive(offset);
        let symbols = self.symbols();
        symbols
            .declarations()
            .iter()
            .find(|declaration| contains(&declaration.identifier))
            .map(Symbol::Declaration)
            .or_else(|| {
                symbols
                    .references()
                    .iter()
                    .find(|reference| contains(&reference.identifier))
                    .map(Symbol::Reference)
            })
    }

    /// Returns the declaration the identifier at `offset` names, or `None` if there's no
    /// identifier there or nothing declares its name.
    ///
    /// A reference names the first declaration of its name. So does a declaration, which is
    /// itself unless the name was already declared.
    pub fn definition(&self, offset: TextSize) -> Option<&Declaration> {
        let symbol = self.symbol_at(offset)?;
        let position = self.symbols().resolve(symbol.identifier())?;
        self.symbols().declarations().get(position)
    }

    /// Returns every place the name of the identifier at `offset` is written, declarations and
    /// references alike, in document order. It's empty when there's no identifier at `offset`.
    pub fn occurrences(&self, offset: TextSize) -> Vec<Occurrence> {
        let Some(name) = self
            .symbol_at(offset)
            .and_then(|symbol| text(symbol.identifier()))
        else {
            return Vec::new();
        };
        let symbols = self.symbols();
        let declarations = symbols
            .declarations()
            .iter()
            .map(|declaration| (&declaration.identifier, true));
        let references = symbols
            .references()
            .iter()
            .map(|reference| (&reference.identifier, false));
        let mut occurrences: Vec<Occurrence> = declarations
            .chain(references)
            .filter(|(identifier, _)| text(identifier).as_deref() == Some(name.as_str()))
            .filter_map(|(identifier, is_declaration)| {
                Some(Occurrence {
                    range: identifier.syntax().text_range(),
                    name: identifier.name()?.text_range(),
                    is_declaration,
                })
            })
            .collect();
        occurrences.sort_by_key(|occurrence| occurrence.range.start());
        occurrences
    }
}

pub(crate) fn nameable<'a>(symbols: &'a Symbols, reference: &Reference) -> Vec<&'a Declaration> {
    let named: Vec<Identifier> = if reference.kind == ReferenceKind::Stance {
        reference
            .identifier
            .syntax()
            .ancestors()
            .find_map(Contest::cast)
            .map(|contest| members(&contest))
            .unwrap_or_default()
    } else {
        symbols
            .declarations()
            .iter()
            .map(|declaration| declaration.identifier.clone())
            .collect()
    };
    let mut positions = Vec::new();
    for identifier in named {
        let Some(position) = symbols.resolve(&identifier) else {
            continue;
        };
        let entity = symbols.declarations()[position].entity;
        if !positions.contains(&position) && reference.kind.accepts(entity) {
            positions.push(position);
        }
    }
    positions
        .into_iter()
        .map(|position| &symbols.declarations()[position])
        .collect()
}

fn members(contest: &Contest) -> Vec<Identifier> {
    contest
        .campaigns()
        .flat_map(|campaign| {
            let members = campaign.members().filter_map(|member| member.identifier());
            let parties = campaign
                .party_members()
                .filter_map(|member| member.identifier());
            members.chain(parties).collect::<Vec<_>>()
        })
        .collect()
}

fn text(identifier: &Identifier) -> Option<String> {
    Some(identifier.name()?.text().to_owned())
}

#[cfg(test)]
mod tests {
    use oval_syntax::ast::AstNode;
    use oval_syntax::{TextRange, TextSize};

    use crate::{Analysis, Symbol};

    const SOURCE: &str = "oval 1\nparty @dem \"Democratic\"\ncandidate @n \"Nadia\"\n  party @dem\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @n\n    campaign party @dem\n    recommend\n      support @n\n      oppose @nobody\n";

    fn at(needle: &str, nth: usize) -> TextSize {
        let offset = SOURCE.match_indices(needle).nth(nth).unwrap().0;
        TextSize::try_from(offset).unwrap()
    }

    fn range(needle: &str, nth: usize) -> TextRange {
        TextRange::at(at(needle, nth), TextSize::of(needle))
    }

    #[test]
    fn the_symbol_at_an_offset_is_the_identifier_it_falls_in_or_just_after() {
        let analysis = Analysis::new(SOURCE).unwrap();
        let start = at("@dem", 0);
        for offset in [
            start,
            start + TextSize::from(1),
            start + TextSize::of("@dem"),
        ] {
            assert!(
                matches!(analysis.symbol_at(offset), Some(Symbol::Declaration(_))),
                "{offset:?}"
            );
        }
        assert!(matches!(
            analysis.symbol_at(at("@dem", 1)),
            Some(Symbol::Reference(_))
        ));
        assert_eq!(analysis.symbol_at(at("party", 0)), None);
        assert_eq!(
            analysis.symbol_at(at("\"Democratic\"", 0) + TextSize::from(2)),
            None
        );
    }

    #[test]
    fn a_definition_is_the_first_declaration_of_the_name() {
        let analysis = Analysis::new(SOURCE).unwrap();
        let declaration_of = |offset| {
            analysis
                .definition(offset)
                .map(|declaration| declaration.identifier.syntax().text_range())
        };
        assert_eq!(declaration_of(at("@n", 1)), Some(range("@n", 0)));
        assert_eq!(declaration_of(at("@n", 2)), Some(range("@n", 0)));
        assert_eq!(declaration_of(at("@dem", 2)), Some(range("@dem", 0)));
        assert_eq!(declaration_of(at("@n", 0)), Some(range("@n", 0)));
        assert_eq!(declaration_of(at("@nobody", 0)), None);
        assert_eq!(declaration_of(at("ballot", 0)), None);
    }

    #[test]
    fn occurrences_are_every_place_a_name_is_written_in_document_order() {
        let analysis = Analysis::new(SOURCE).unwrap();
        let occurrences = analysis.occurrences(at("@dem", 2));
        let found: Vec<(TextRange, TextRange, bool)> = occurrences
            .iter()
            .map(|occurrence| (occurrence.range, occurrence.name, occurrence.is_declaration))
            .collect();
        let name = |nth| TextRange::at(at("@dem", nth) + TextSize::from(1), TextSize::of("dem"));
        assert_eq!(
            found,
            [
                (range("@dem", 0), name(0), true),
                (range("@dem", 1), name(1), false),
                (range("@dem", 2), name(2), false),
            ]
        );
        assert_eq!(analysis.occurrences(at("@nobody", 0)).len(), 1);
        let later = "oval 1\ncandidate @c \"C\"\n  party @grn\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign party @grn \"Green\"\n";
        let analysis_later = Analysis::new(later).unwrap();
        let first = TextSize::try_from(later.find("@grn").unwrap()).unwrap();
        let declarations: Vec<bool> = analysis_later
            .occurrences(first)
            .iter()
            .map(|occurrence| occurrence.is_declaration)
            .collect();
        assert_eq!(declarations, [false, true]);
        assert_eq!(analysis.occurrences(at("ballot", 0)), []);
    }

    #[test]
    fn a_document_with_errors_still_has_symbols() {
        let source = "oval 1\ncandidate @n \"N\n  party @dem\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @n\n";
        let analysis = Analysis::new(source).unwrap();
        let offset = TextSize::try_from(source.rfind("@n").unwrap()).unwrap();
        assert!(analysis.definition(offset).is_some());
    }
}
