//! Contests, campaigns, and their members.
//!
//! A [`Contest`] is one office (Section 2.13). Its voting method is a [`ContestKind`], its
//! [`Campaign`]s are the choices in ballot order, and its recommendation is a
//! [`ContestRecommend`]. A campaign has no identifier of its own; it is one or more [`Member`]s
//! running together, each a candidate reference or an inline candidate declaration (Section 2.14).

use super::{
    ContestRecommend, Identifier, MarkdownField, PartyReference, StringField, UrlField, ast_node,
    block, block_children, child, field, has_token, identifier, integer, string, token_range,
    tokens,
};
use crate::TextRange;
use crate::{Keyword, SyntaxKind};

ast_node! {
    /// One office chosen among campaigns, written as `contest`.
    ///
    /// A contest requires a block (`missing-block`). The grammar accepts any number of campaigns
    /// and recommendations in it so that a half-written contest still parses; validation requires
    /// at least one campaign outside a retention contest (`contest-without-campaigns`) and exactly
    /// one recommendation (`missing-recommendation`, `repeated-recommendation`).
    ///
    /// # Examples
    ///
    /// ```
    /// use oval_syntax::ast::{AstNode, BallotItem, ContestKind, Document};
    /// use oval_syntax::parser::parse;
    ///
    /// let source = r#"oval 1
    /// ballot @city "City"
    ///   contest @council "City Council" multiple 3
    ///     campaign incumbent @delgado "Rosa Delgado"
    ///     campaign @okafor "Ada Okafor"
    ///     recommend
    ///       support @okafor @delgado
    /// "#;
    /// let result = parse(source).unwrap();
    /// let document = Document::cast(result.syntax()).unwrap();
    /// let Some(BallotItem::Contest(contest)) = document.ballots().next().unwrap().items().next()
    /// else {
    ///     panic!("the first item is a contest");
    /// };
    /// assert_eq!(contest.kind(), Some(ContestKind::Multiple { seats: Some(3) }));
    /// let campaigns: Vec<_> = contest.campaigns().collect();
    /// assert!(campaigns[0].is_incumbent());
    /// let second = campaigns[1].members().next().and_then(|m| m.name());
    /// assert_eq!(second.as_deref(), Some("Ada Okafor"));
    /// ```
    Contest
}

ast_node! {
    /// One or more candidates running together, written as `campaign`.
    ///
    /// A campaign is written in one of two forms (Section 2.14). In the one-line form the single
    /// member is on the header line and the block carries the campaign's fields, plus the
    /// candidate's `designation` and `party` when the member is declared there. In the block form
    /// the members are lines in the block, each with its own block of candidate fields, and the
    /// first member is the head of the ticket.
    Campaign
}

ast_node! {
    /// A member of a campaign, which is a candidate reference or inline declaration, with an
    /// optional role.
    ///
    /// `@jlee` refers to a candidate declared elsewhere; `@jlee "Jordan Lee"` declares the
    /// candidate here. Either may be followed by `as "Role"`. Only a declaration may carry a block
    /// of candidate fields (`block-on-reference`).
    Member
}

/// The voting method of a contest, as written on its header line.
///
/// The variants follow the table in Section 2.13. The arguments are read as written; whether
/// they are in range is validation (`invalid-multiple-count`, `invalid-ranked-max`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContestKind {
    /// A vote for one, written `single`.
    Single,
    /// A vote for up to `seats`, with that many elected, written `multiple` _n_.
    ///
    /// `seats` is `None` when the count is missing or malformed, which the parser reports.
    Multiple {
        /// The number of campaigns the voter may vote for.
        seats: Option<u32>,
    },
    /// A ranking of choices, written `ranked`, optionally with `max` _n_.
    ///
    /// `max` is `None` when the ballot does not limit how many may be ranked, and also when
    /// `max` was written without a count, which the parser reports.
    Ranked {
        /// The number of campaigns the voter may rank, when the ballot limits it.
        max: Option<u32>,
    },
    /// A yes or no on a single officeholder, written `retention`.
    Retention,
}

impl Contest {
    /// The declared identifier.
    ///
    /// `None` when the parser recovered from a line with no identifier or an `@` with no word.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The contest as named on the ballot.
    ///
    /// `None` when the line ended before the title.
    pub fn title(&self) -> Option<String> {
        string(&self.0)
    }

    /// The voting method.
    ///
    /// `None` when the line ended before the kind or the parser recovered from a word that is not
    /// one, which it reports as `missing-contest-kind` or `unexpected-token`.
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

    /// The range of the integer after `multiple` or `max`, when there is one.
    ///
    /// Diagnostics about the count, such as `invalid-multiple-count`, point here. `None` for
    /// `single`, `retention`, `ranked` without `max`, and a count that was not written.
    pub fn kind_argument_range(&self) -> Option<TextRange> {
        let node = self
            .0
            .children()
            .find(|n| n.kind() == SyntaxKind::ContestKind)?;
        token_range(&node, SyntaxKind::IntegerLiteral)
    }

    /// Instructions or a second line of the title as printed, from the `subtitle` field.
    pub fn subtitle(&self) -> Option<StringField> {
        field(&self.0, Keyword::Subtitle)
    }

    /// How the votes are counted, in prose, from the `tabulation` field.
    pub fn tabulation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Tabulation)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// The campaigns, in ballot order.
    ///
    /// Empty when the contest has no block or no `campaign` in it.
    pub fn campaigns(&self) -> impl Iterator<Item = Campaign> + use<> {
        block_children(&self.0)
    }

    /// Every `recommend` in the block, in source order.
    ///
    /// A valid contest has exactly one; this returns them all so that validation can report a
    /// second one where it is written.
    pub fn recommendations(&self) -> impl Iterator<Item = ContestRecommend> + use<> {
        block_children(&self.0)
    }
}

impl Campaign {
    /// Whether the `incumbent` flag is set, meaning the campaign currently occupies the office.
    pub fn is_incumbent(&self) -> bool {
        self.has_flag(SyntaxKind::IncumbentKeyword)
    }

    /// Whether the `withdrawn` flag is set, meaning the campaign has ended but remains on the
    /// ballot.
    pub fn is_withdrawn(&self) -> bool {
        self.has_flag(SyntaxKind::WithdrawnKeyword)
    }

    /// The members, head of ticket first, meaning the header-line member and then any in the
    /// block.
    ///
    /// A well-formed campaign has its member on the header line or its members in the block, not
    /// both. Empty when neither is written, which validation reports as
    /// `campaign-without-members`.
    pub fn members(&self) -> impl Iterator<Item = Member> + use<> {
        let header = child::<Member>(&self.0);
        header.into_iter().chain(block_children(&self.0))
    }

    /// The campaign's site, from the `website` field.
    pub fn website(&self) -> Option<UrlField> {
        field(&self.0, Keyword::Website)
    }

    /// The campaign's note, as distinct from a member's, from the `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// The `designation` field, permitted only in the one-line form with an inline candidate.
    ///
    /// It is the candidate's designation, written at the campaign level because the one-line form
    /// has nowhere else to put it. Anywhere else it is `misplaced-candidate-field`.
    pub fn designation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Designation)
    }

    /// The `party` line, permitted only in the one-line form with an inline candidate.
    ///
    /// It is the candidate's party, written at the campaign level because the one-line form has
    /// nowhere else to put it. Anywhere else it is `misplaced-candidate-field`.
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
    ///
    /// `None` when the parser recovered from an `@` with no word after it.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The candidate's name, present only when this member declares the candidate.
    pub fn name(&self) -> Option<String> {
        self.strings().0
    }

    /// Whether this member declares the candidate rather than referring to one.
    ///
    /// A declaration is an identifier followed by a string (Section 2.7). The string decides;
    /// whether the identifier is also declared elsewhere is the validation rule
    /// `duplicate-declaration`.
    pub fn is_declaration(&self) -> bool {
        self.name().is_some()
    }

    /// The role on the ticket, from `as "Role"`.
    ///
    /// `None` when no role is written, which is usual for a campaign of one.
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
