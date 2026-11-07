//! The index of a document, built once before validation so that every rule can resolve
//! references and walk the constructs without searching the tree again.

use std::collections::HashMap;

use oval_syntax::TextRange;
use oval_syntax::ast::{
    AstNode, Ballot, BallotItem, Campaign, Contest, Document, Identifier, Measure, Member,
    PartyReference, Section, SectionItem,
};

/// The kind of construct an identifier declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EntityKind {
    /// A `party` declaration, at the top level or inline.
    Party,
    /// A `candidate` declaration, at the top level or inline in a campaign.
    Candidate,
    /// A `ballot`, root or nested.
    Ballot,
    /// A `contest`.
    Contest,
    /// A `measure`.
    Measure,
}

impl EntityKind {
    /// The lowercase noun for this kind, as diagnostic messages use it.
    pub(crate) fn noun(self) -> &'static str {
        match self {
            Self::Party => "party",
            Self::Candidate => "candidate",
            Self::Ballot => "ballot",
            Self::Contest => "contest",
            Self::Measure => "measure",
        }
    }
}

/// One declared identifier, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Declaration {
    /// The identifier without its `@`.
    pub(crate) name: String,
    /// The kind of construct declared.
    pub(crate) kind: EntityKind,
    /// The range of the identifier token, `@` included.
    pub(crate) range: TextRange,
}

/// The position a reference appears in, which decides what it may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ReferenceSite {
    /// A `party` line on a candidate or campaign, which names a party.
    PartyLine,
    /// A member of a campaign, which names a candidate.
    Member,
    /// A `support` or `oppose` line, which names a candidate.
    Stance,
}

/// One use of an identifier, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reference {
    /// The identifier without its `@`.
    pub(crate) name: String,
    /// The kind of declaration the position accepts.
    pub(crate) expected: EntityKind,
    /// The position the reference appears in.
    pub(crate) site: ReferenceSite,
    /// The range of the identifier token, `@` included.
    pub(crate) range: TextRange,
}

/// Everything the validation rules look up about a document.
///
/// The index is built once by [`Index::build`] and read by every rule. Declarations and
/// references keep document order, so diagnostics derived from them come out in source order.
#[derive(Debug, Default)]
pub(crate) struct Index {
    /// Every declared identifier, in document order.
    pub(crate) declarations: Vec<Declaration>,
    /// Every use of an identifier, in document order.
    pub(crate) references: Vec<Reference>,
    /// The position in `declarations` of the first declaration of each name.
    pub(crate) by_name: HashMap<String, usize>,
    /// The positions in `declarations` of every declaration whose name was already declared.
    pub(crate) duplicates: Vec<usize>,
    /// Every contest, in ballot order.
    pub(crate) contests: Vec<Contest>,
    /// Every measure, in ballot order.
    pub(crate) measures: Vec<Measure>,
    /// Every ballot, root and nested, in ballot order.
    pub(crate) ballots: Vec<Ballot>,
    /// Every section, in ballot order.
    pub(crate) sections: Vec<Section>,
    /// Every `party` line that declares a party inline rather than naming one.
    pub(crate) inline_parties: Vec<PartyReference>,
}

impl Index {
    /// Walks `document` once and records its declarations, references, and constructs.
    pub(crate) fn build(document: &Document) -> Self {
        let mut index = Self::default();
        index.collect(document);
        for (i, declaration) in index.declarations.iter().enumerate() {
            if index.by_name.contains_key(&declaration.name) {
                index.duplicates.push(i);
            } else {
                index.by_name.insert(declaration.name.clone(), i);
            }
        }
        index
    }

    /// The first declaration of `name`, or `None` when nothing declares it.
    pub(crate) fn resolve(&self, name: &str) -> Option<&Declaration> {
        self.by_name.get(name).map(|i| &self.declarations[*i])
    }

    fn collect(&mut self, document: &Document) {
        for party in document.parties() {
            self.declare(party.id(), EntityKind::Party);
        }
        for candidate in document.candidates() {
            self.declare(candidate.id(), EntityKind::Candidate);
            if let Some(party) = candidate.party() {
                self.party_reference(&party);
            }
        }
        for ballot in document.ballots() {
            self.ballot(&ballot);
        }
    }

    fn ballot(&mut self, ballot: &Ballot) {
        self.ballots.push(ballot.clone());
        self.declare(ballot.id(), EntityKind::Ballot);
        for item in ballot.items() {
            match item {
                BallotItem::Section(section) => {
                    self.sections.push(section.clone());
                    for item in section.items() {
                        match item {
                            SectionItem::Contest(contest) => self.contest(&contest),
                            SectionItem::Measure(measure) => self.measure(&measure),
                        }
                    }
                }
                BallotItem::Contest(contest) => self.contest(&contest),
                BallotItem::Measure(measure) => self.measure(&measure),
                BallotItem::Ballot(sub) => self.ballot(&sub),
            }
        }
    }

    fn contest(&mut self, contest: &Contest) {
        self.contests.push(contest.clone());
        self.declare(contest.id(), EntityKind::Contest);
        for campaign in contest.campaigns() {
            self.campaign(&campaign);
        }
        for recommendation in contest.recommendations() {
            let named = recommendation
                .support()
                .flat_map(|tier| tier.candidates().collect::<Vec<_>>())
                .chain(
                    recommendation
                        .oppose()
                        .flat_map(|o| o.candidates().collect::<Vec<_>>()),
                );
            for identifier in named {
                self.refer(&identifier, EntityKind::Candidate, ReferenceSite::Stance);
            }
        }
    }

    fn campaign(&mut self, campaign: &Campaign) {
        for member in campaign.members() {
            self.member(&member);
        }
        if let Some(party) = campaign.party() {
            self.party_reference(&party);
        }
    }

    fn member(&mut self, member: &Member) {
        if member.is_declaration() {
            self.declare(member.id(), EntityKind::Candidate);
        } else if let Some(id) = member.id() {
            self.refer(&id, EntityKind::Candidate, ReferenceSite::Member);
        }
        if let Some(party) = member.party() {
            self.party_reference(&party);
        }
    }

    fn measure(&mut self, measure: &Measure) {
        self.measures.push(measure.clone());
        self.declare(measure.id(), EntityKind::Measure);
    }

    fn party_reference(&mut self, party: &PartyReference) {
        if party.is_declaration() {
            self.inline_parties.push(party.clone());
            self.declare(party.id(), EntityKind::Party);
        } else if let Some(id) = party.id() {
            self.refer(&id, EntityKind::Party, ReferenceSite::PartyLine);
        }
    }

    fn declare(&mut self, id: Option<Identifier>, kind: EntityKind) {
        if let Some(id) = id {
            self.declarations.push(Declaration {
                name: id.name(),
                kind,
                range: id.range(),
            });
        }
    }

    fn refer(&mut self, id: &Identifier, expected: EntityKind, site: ReferenceSite) {
        self.references.push(Reference {
            name: id.name(),
            expected,
            site,
            range: id.range(),
        });
    }
}
