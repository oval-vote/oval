use std::collections::HashMap;

use oval_syntax::ast::{AstNode, Document, Identifier, Member, PartyMember, PartyRef};
use oval_syntax::{SyntaxKind, SyntaxNode};

use super::kept_nodes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Entity {
    Party,
    Candidate,
    Ballot,
    Contest,
    Measure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Position {
    PartyField,
    CandidateMember,
    PartyMember,
    Stance,
}

impl Position {
    pub(super) fn accepts(self, entity: Entity) -> bool {
        match self {
            Self::PartyField | Self::PartyMember => entity == Entity::Party,
            Self::CandidateMember => entity == Entity::Candidate,
            Self::Stance => matches!(entity, Entity::Candidate | Entity::Party),
        }
    }

    pub(super) fn is_use(self) -> bool {
        self != Self::Stance
    }
}

pub(super) struct Declaration {
    pub(super) entity: Entity,
    pub(super) identifier: Identifier,
}

pub(super) struct Reference {
    pub(super) position: Position,
    pub(super) identifier: Identifier,
}

pub(super) struct Index {
    pub(super) declarations: Vec<Declaration>,
    pub(super) references: Vec<Reference>,
    first: HashMap<String, usize>,
}

impl Index {
    pub(super) fn new(document: &Document) -> Self {
        let mut index = Self {
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
                    index
                        .first
                        .entry(name.text().to_owned())
                        .or_insert(index.declarations.len());
                    index.declarations.push(Declaration { entity, identifier });
                }
                Some(Role::Reference(position)) => {
                    index.references.push(Reference {
                        position,
                        identifier,
                    });
                }
                None => {}
            }
        }
        index
    }

    pub(super) fn resolve(&self, identifier: &Identifier) -> Option<usize> {
        let name = identifier.name()?;
        self.first.get(name.text()).copied()
    }
}

enum Role {
    Declaration(Entity),
    Reference(Position),
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
                Role::Reference(Position::CandidateMember)
            }
        }
        SyntaxKind::PartyMember => {
            if PartyMember::cast(node)?.is_declaration() {
                Role::Declaration(Entity::Party)
            } else {
                Role::Reference(Position::PartyMember)
            }
        }
        SyntaxKind::PartyRef => {
            if PartyRef::cast(node)?.is_declaration() {
                Role::Declaration(Entity::Party)
            } else {
                Role::Reference(Position::PartyField)
            }
        }
        SyntaxKind::Support | SyntaxKind::Oppose => Role::Reference(Position::Stance),
        _ => return None,
    };
    Some(role)
}
