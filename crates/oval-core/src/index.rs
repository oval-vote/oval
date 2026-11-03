use std::collections::HashMap;

use oval_syntax::TextRange;
use oval_syntax::ast::{
    Ballot, BallotItem, Campaign, Contest, Document, Identifier, Measure, Member, PartyReference,
    Section, SectionItem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EntityKind {
    Party,
    Candidate,
    Ballot,
    Contest,
    Measure,
}

impl EntityKind {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Declaration {
    pub(crate) name: String,
    pub(crate) kind: EntityKind,
    pub(crate) range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reference {
    pub(crate) name: String,
    pub(crate) expected: EntityKind,
    pub(crate) range: TextRange,
}

#[derive(Debug, Default)]
pub(crate) struct Index {
    pub(crate) declarations: Vec<Declaration>,
    pub(crate) references: Vec<Reference>,
    pub(crate) by_name: HashMap<String, usize>,
    pub(crate) duplicates: Vec<usize>,
    pub(crate) contests: Vec<Contest>,
    pub(crate) measures: Vec<Measure>,
    pub(crate) ballots: Vec<Ballot>,
    pub(crate) sections: Vec<Section>,
}

impl Index {
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
                self.refer(&identifier, EntityKind::Candidate);
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
            self.refer(&id, EntityKind::Candidate);
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
            self.declare(party.id(), EntityKind::Party);
        } else if let Some(id) = party.id() {
            self.refer(&id, EntityKind::Party);
        }
    }

    fn declare(&mut self, id: Option<Identifier>, kind: EntityKind) {
        if let Some(id) = id {
            self.declarations.push(Declaration {
                name: id.name().to_owned(),
                kind,
                range: id.range(),
            });
        }
    }

    fn refer(&mut self, id: &Identifier, expected: EntityKind) {
        self.references.push(Reference {
            name: id.name().to_owned(),
            expected,
            range: id.range(),
        });
    }
}
