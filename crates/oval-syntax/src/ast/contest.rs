use super::{
    ContestRecommend, Identifier, MarkdownField, PartyReference, StringField, UrlField, ast_node,
    block, block_children, child, field, has_token, identifier, integer, string, tokens,
};
use crate::{Keyword, SyntaxKind};

ast_node! {
    /// A `contest`: one office, chosen among campaigns.
    Contest
}

ast_node! {
    /// A `campaign`: one or more candidates running together.
    Campaign
}

ast_node! {
    /// A member of a campaign: a candidate reference or inline declaration, with an optional role.
    Member
}

/// The voting method of a contest, as written on its header line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContestKind {
    /// Vote for one.
    Single,
    /// Vote for up to `seats`; `None` when the count is missing or malformed.
    Multiple {
        /// How many campaigns the voter may vote for.
        seats: Option<u32>,
    },
    /// Rank choices, optionally capped at `max` rankings.
    Ranked {
        /// How many campaigns the voter may rank, when the ballot limits it.
        max: Option<u32>,
    },
    /// Yes or no on a single officeholder.
    Retention,
}

impl Contest {
    /// The declared identifier.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The contest as named on the ballot.
    pub fn title(&self) -> Option<String> {
        string(&self.0)
    }

    /// The voting method.
    pub fn kind(&self) -> Option<ContestKind> {
        let node = self
            .0
            .children()
            .find(|n| n.kind() == SyntaxKind::ContestKind)?;
        let keyword = tokens(&node).find_map(|t| t.kind().as_keyword())?;
        Some(match keyword {
            Keyword::Single => ContestKind::Single,
            Keyword::Multiple => ContestKind::Multiple {
                seats: integer(&node),
            },
            Keyword::Ranked => ContestKind::Ranked {
                max: has_token(&node, SyntaxKind::MaxKeyword)
                    .then(|| integer(&node))
                    .flatten(),
            },
            Keyword::Retention => ContestKind::Retention,
            _ => return None,
        })
    }

    /// The `subtitle` field.
    pub fn subtitle(&self) -> Option<StringField> {
        field(&self.0, Keyword::Subtitle)
    }

    /// The `tabulation` field.
    pub fn tabulation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Tabulation)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// The campaigns, in ballot order.
    pub fn campaigns(&self) -> impl Iterator<Item = Campaign> + use<> {
        block_children(&self.0)
    }

    /// Every `recommend` in the block. A valid contest has exactly one.
    pub fn recommendations(&self) -> impl Iterator<Item = ContestRecommend> + use<> {
        block_children(&self.0)
    }
}

impl Campaign {
    /// Whether the `incumbent` flag is set.
    pub fn is_incumbent(&self) -> bool {
        self.has_flag(SyntaxKind::IncumbentKeyword)
    }

    /// Whether the `withdrawn` flag is set.
    pub fn is_withdrawn(&self) -> bool {
        self.has_flag(SyntaxKind::WithdrawnKeyword)
    }

    /// The members, head of ticket first: the header-line member, then any in the block.
    pub fn members(&self) -> impl Iterator<Item = Member> + use<> {
        let header = child::<Member>(&self.0);
        header.into_iter().chain(block_children(&self.0))
    }

    /// The `website` field.
    pub fn website(&self) -> Option<UrlField> {
        field(&self.0, Keyword::Website)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// The `designation` field, permitted only in the one-line form with an inline candidate.
    pub fn designation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Designation)
    }

    /// The `party` line, permitted only in the one-line form with an inline candidate.
    pub fn party(&self) -> Option<PartyReference> {
        block(&self.0).and_then(|b| child(&b))
    }

    fn has_flag(&self, keyword: SyntaxKind) -> bool {
        self.0
            .children()
            .filter(|n| n.kind() == SyntaxKind::Flag)
            .any(|flag| has_token(&flag, keyword))
    }
}

impl Member {
    /// The candidate referred to or declared.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The candidate's name, present only when this member declares the candidate.
    pub fn name(&self) -> Option<String> {
        self.strings().0
    }

    /// Whether this member declares the candidate rather than referring to one.
    pub fn is_declaration(&self) -> bool {
        self.name().is_some()
    }

    /// The role on the ticket, from `as "Role"`.
    pub fn role(&self) -> Option<String> {
        self.strings().1
    }

    /// The `designation` field of an inline declaration.
    pub fn designation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Designation)
    }

    /// The `party` line of an inline declaration.
    pub fn party(&self) -> Option<PartyReference> {
        block(&self.0).and_then(|b| child(&b))
    }

    /// The `note` field of an inline declaration.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    fn strings(&self) -> (Option<String>, Option<String>) {
        let mut name = None;
        let mut role = None;
        let mut after_as = false;
        for t in tokens(&self.0) {
            match t.kind() {
                SyntaxKind::AsKeyword => after_as = true,
                SyntaxKind::StringLiteral if after_as => role = Some(super::string_value(t.text())),
                SyntaxKind::StringLiteral => name = Some(super::string_value(t.text())),
                _ => {}
            }
        }
        (name, role)
    }
}
