use std::collections::HashMap;

use oval_syntax::ast::{
    AstNode, AstToken, Document, Identifier, Member, PartyMember, PartyRef, StringLiteral,
};
use oval_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

use crate::validate::kept_nodes;

/// The kind of thing a declaration declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Entity {
    /// A party, declared at the top level, by a `party` field, or as a party campaign's member.
    Party,
    /// A candidate, declared at the top level or as a campaign's member.
    Candidate,
    /// A root ballot or a sub-ballot.
    Ballot,
    /// A contest.
    Contest,
    /// A measure.
    Measure,
}

/// Where a reference is written, which decides the kinds of declaration it may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// A `party` field, as in `party @dem`.
    PartyField,
    /// A member of a candidate campaign, as in `campaign @jlee`.
    CandidateMember,
    /// A member of a party campaign, as in `campaign party @dem`.
    PartyMember,
    /// A choice in a `support` or `oppose` line, which names a campaign by one of its members.
    Stance,
}

impl ReferenceKind {
    /// Returns `true` if a reference of this kind may name a declaration of `entity`.
    ///
    /// ```
    /// use oval_core::{Entity, ReferenceKind};
    ///
    /// assert!(ReferenceKind::PartyField.accepts(Entity::Party));
    /// assert!(ReferenceKind::Stance.accepts(Entity::Candidate));
    /// assert!(!ReferenceKind::CandidateMember.accepts(Entity::Party));
    /// ```
    pub fn accepts(self, entity: Entity) -> bool {
        match self {
            Self::PartyField | Self::PartyMember => entity == Entity::Party,
            Self::CandidateMember => entity == Entity::Candidate,
            Self::Stance => matches!(entity, Entity::Candidate | Entity::Party),
        }
    }

    /// Returns `true` if a reference of this kind uses the declaration it names.
    ///
    /// A `party` field or a campaign member uses its declaration. A `support` or `oppose` choice
    /// only points at a campaign that already uses it, so it doesn't count.
    pub fn is_use(self) -> bool {
        self != Self::Stance
    }
}

/// A declared identifier, and what it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// The kind of thing declared.
    pub entity: Entity,
    /// The identifier as written in the declaration.
    pub identifier: Identifier,
}

impl Declaration {
    /// Returns the name the declaration gives, the string after its identifier, or `None` if it
    /// has none.
    pub fn name(&self) -> Option<StringLiteral> {
        self.identifier
            .syntax()
            .parent()?
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find_map(StringLiteral::cast)
    }
}

/// A use of an identifier where a declaration is expected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// Where the reference is written.
    pub kind: ReferenceKind,
    /// The identifier as written in the reference.
    pub identifier: Identifier,
}

/// Every declaration in a document and every reference to one, and how each reference resolves.
///
/// Identifiers share one namespace across every kind of declaration. A reference resolves to the
/// first declaration of its name in document order, even when the name is declared again later
/// or the declaration is of a kind the reference doesn't accept. Parts of the document the parser
/// set aside as errors are skipped.
///
/// ```
/// use oval_core::{Entity, ReferenceKind, Symbols};
///
/// let report = oval_core::check("oval 1\nparty @dem \"Democratic\"\ncandidate @n \"N\"\n  party @dem\n")?;
/// let symbols = Symbols::new(&report.parse().document());
///
/// let entities: Vec<Entity> = symbols.declarations().iter().map(|d| d.entity).collect();
/// assert_eq!(entities, [Entity::Party, Entity::Candidate]);
///
/// let reference = &symbols.references()[0];
/// assert_eq!(reference.kind, ReferenceKind::PartyField);
/// assert_eq!(symbols.resolve(&reference.identifier), Some(0));
/// # Ok::<(), oval_core::InputRejection>(())
/// ```
#[derive(Debug, Clone)]
pub struct Symbols {
    declarations: Vec<Declaration>,
    references: Vec<Reference>,
    first: HashMap<String, usize>,
}

impl Symbols {
    /// Collects the declarations and references of `document`, each in document order.
    pub fn new(document: &Document) -> Self {
        let mut symbols = Self {
            declarations: Vec::new(),
            references: Vec::new(),
            first: HashMap::new(),
        };
        for identifier in kept_nodes(document.syntax()).filter_map(Identifier::cast) {
            let Some(name) = identifier.name() else {
                continue;
            };
            match identifier.syntax().parent().and_then(role) {
                Some(Role::Declaration(entity)) => {
                    symbols
                        .first
                        .entry(name.text().to_owned())
                        .or_insert(symbols.declarations.len());
                    symbols
                        .declarations
                        .push(Declaration { entity, identifier });
                }
                Some(Role::Reference(kind)) => {
                    symbols.references.push(Reference { kind, identifier });
                }
                None => {}
            }
        }
        symbols
    }

    /// Returns every declaration, in document order.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Returns every reference, in document order.
    pub fn references(&self) -> &[Reference] {
        &self.references
    }

    /// Returns the position in [`declarations`](Self::declarations) of the first declaration of
    /// `identifier`'s name, or `None` if nothing declares it.
    ///
    /// The identifier may be a reference or a declaration. A later declaration of a name that is
    /// already declared resolves to the earlier one.
    pub fn resolve(&self, identifier: &Identifier) -> Option<usize> {
        let name = identifier.name()?;
        self.first.get(name.text()).copied()
    }
}

enum Role {
    Declaration(Entity),
    Reference(ReferenceKind),
}

fn role(node: SyntaxNode) -> Option<Role> {
    let role = match node.kind() {
        SyntaxKind::Party => Role::Declaration(Entity::Party),
        SyntaxKind::Candidate => Role::Declaration(Entity::Candidate),
        SyntaxKind::Ballot => Role::Declaration(Entity::Ballot),
        SyntaxKind::Contest => Role::Declaration(Entity::Contest),
        SyntaxKind::Measure => Role::Declaration(Entity::Measure),
        SyntaxKind::Member => {
            if Member::cast(node)?.is_declaration() {
                Role::Declaration(Entity::Candidate)
            } else {
                Role::Reference(ReferenceKind::CandidateMember)
            }
        }
        SyntaxKind::PartyMember => {
            if PartyMember::cast(node)?.is_declaration() {
                Role::Declaration(Entity::Party)
            } else {
                Role::Reference(ReferenceKind::PartyMember)
            }
        }
        SyntaxKind::PartyRef => {
            if PartyRef::cast(node)?.is_declaration() {
                Role::Declaration(Entity::Party)
            } else {
                Role::Reference(ReferenceKind::PartyField)
            }
        }
        SyntaxKind::Support | SyntaxKind::Oppose => Role::Reference(ReferenceKind::Stance),
        _ => return None,
    };
    Some(role)
}

#[cfg(test)]
mod tests {
    use super::{Entity, ReferenceKind, Symbols};

    fn symbols(text: &str) -> Symbols {
        Symbols::new(&oval_syntax::parse(text).unwrap().document())
    }

    fn name(identifier: &oval_syntax::ast::Identifier) -> String {
        identifier.name().unwrap().text().to_owned()
    }

    const DOCUMENT: &str = "oval 1\nparty @dem \"Democratic\"\ncandidate @n \"Nadia\"\n  party @dem\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @n\n    campaign\n      @c \"Carl\"\n        party @grn \"Green\"\n      @n\n    campaign party @dem\n    campaign party\n      @lab \"Labour\"\n    recommend\n      support @c\n      oppose @lab\n  measure @m \"M\"\n    recommend yes\n";

    #[test]
    fn declarations_are_in_document_order_with_what_they_declare() {
        let symbols = symbols(DOCUMENT);
        let declarations: Vec<(String, Entity)> = symbols
            .declarations()
            .iter()
            .map(|declaration| (name(&declaration.identifier), declaration.entity))
            .collect();
        assert_eq!(
            declarations,
            [
                ("dem".to_owned(), Entity::Party),
                ("n".to_owned(), Entity::Candidate),
                ("b".to_owned(), Entity::Ballot),
                ("k".to_owned(), Entity::Contest),
                ("c".to_owned(), Entity::Candidate),
                ("grn".to_owned(), Entity::Party),
                ("lab".to_owned(), Entity::Party),
                ("m".to_owned(), Entity::Measure),
            ]
        );
    }

    #[test]
    fn a_declaration_has_the_name_written_after_its_identifier() {
        let symbols = symbols(DOCUMENT);
        let names: Vec<String> = symbols
            .declarations()
            .iter()
            .map(|declaration| declaration.name().unwrap().value().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "Democratic",
                "Nadia",
                "B",
                "K",
                "Carl",
                "Green",
                "Labour",
                "M"
            ]
        );
    }

    #[test]
    fn references_are_in_document_order_with_where_they_are_written() {
        let symbols = symbols(DOCUMENT);
        let references: Vec<(String, ReferenceKind)> = symbols
            .references()
            .iter()
            .map(|reference| (name(&reference.identifier), reference.kind))
            .collect();
        assert_eq!(
            references,
            [
                ("dem".to_owned(), ReferenceKind::PartyField),
                ("n".to_owned(), ReferenceKind::CandidateMember),
                ("n".to_owned(), ReferenceKind::CandidateMember),
                ("dem".to_owned(), ReferenceKind::PartyMember),
                ("c".to_owned(), ReferenceKind::Stance),
                ("lab".to_owned(), ReferenceKind::Stance),
            ]
        );
    }

    #[test]
    fn a_name_resolves_to_its_first_declaration() {
        let symbols = symbols(
            "oval 1
candidate @x \"First\"
party @x \"Second\"
candidate @y \"Y\"
  party @x
ballot @b \"B\"
  contest @k \"K\" single
    campaign @nobody
    recommend no-position
",
        );
        let resolved: Vec<Option<usize>> = symbols
            .references()
            .iter()
            .map(|reference| symbols.resolve(&reference.identifier))
            .collect();
        assert_eq!(resolved, [Some(0), None]);
        assert_eq!(
            symbols.resolve(&symbols.declarations()[1].identifier),
            Some(0)
        );
    }

    #[test]
    fn identifiers_the_parser_set_aside_are_skipped() {
        let symbols = symbols(
            "oval 1
candidate @c \"C\"
  party @dem \"Dem\"
  party @grn \"Green\"
",
        );
        let names: Vec<String> = symbols
            .declarations()
            .iter()
            .map(|declaration| name(&declaration.identifier))
            .collect();
        assert_eq!(names, ["c", "dem"]);
    }

    #[test]
    fn only_a_party_field_or_a_campaign_member_uses_what_it_names() {
        assert!(ReferenceKind::PartyField.is_use());
        assert!(ReferenceKind::CandidateMember.is_use());
        assert!(ReferenceKind::PartyMember.is_use());
        assert!(!ReferenceKind::Stance.is_use());
    }
}
