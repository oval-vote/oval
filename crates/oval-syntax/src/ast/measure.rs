//! Measures and their recommendations.
//!
//! A [`Measure`] is a yes-or-no question (Section 2.16). Its fields are the official ballot text
//! and the author's research, and its [`MeasureRecommend`] is a [`Position`] (Section 2.17).
//! [`StringListField`] is the type of `supporters` and `opponents`, the two fields whose value may
//! be `none`.

use super::{
    Identifier, MarkdownField, StringField, ast_node, block_children, field, has_token, identifier,
    keyword, string, string_value, tokens,
};
use crate::{Keyword, SyntaxKind};

ast_node! {
    /// A yes-or-no question, written as `measure`.
    ///
    /// A measure requires a block (`missing-block`), and a valid measure has exactly one
    /// recommendation in it (`missing-recommendation`, `repeated-recommendation`).
    ///
    /// # Examples
    ///
    /// ```
    /// use oval_syntax::ast::{AstNode, BallotItem, Document, Position};
    /// use oval_syntax::parser::parse;
    ///
    /// let source = r#"oval 1
    /// ballot @city "City"
    ///   measure @a "Proposition A"
    ///     supporters "Housing Coalition", "Labor Federation"
    ///     opponents none
    ///     recommend yes
    /// "#;
    /// let result = parse(source).unwrap();
    /// let document = Document::cast(result.syntax()).unwrap();
    /// let Some(BallotItem::Measure(measure)) = document.ballots().next().unwrap().items().next()
    /// else {
    ///     panic!("the first item is a measure");
    /// };
    /// assert_eq!(measure.supporters().and_then(|f| f.value()).map(|v| v.len()), Some(2));
    /// assert_eq!(measure.opponents().and_then(|f| f.value()), Some(vec![]));
    /// let recommend = measure.recommendations().next().unwrap();
    /// assert_eq!(recommend.position(), Some(Position::Yes));
    /// ```
    Measure
}

ast_node! {
    /// A measure's `recommend`, which is a position, optionally with a block of `justification`
    /// and `note`.
    MeasureRecommend
}

ast_node! {
    /// A field whose value is a list of strings or `none`, which is to say `supporters` or
    /// `opponents`.
    ///
    /// Such a field has three states (Section 2.16).
    ///
    /// - Absent, which means not researched.
    /// - `none`, which means researched and none known.
    /// - A list of one or more strings.
    StringListField
}

/// The author's position on a measure, which is yes, no, or no position.
///
/// The three spellings of `MeasurePosition` in Section 2.17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Position {
    /// A recommendation to vote yes, written `yes`.
    Yes,
    /// A recommendation to vote no, written `no`.
    No,
    /// A deliberate decision to take no position, written `no-position`.
    NoPosition,
}

impl Measure {
    /// The declared identifier.
    ///
    /// `None` when the parser recovered from a line with no identifier or an `@` with no word.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The measure as named on the ballot.
    ///
    /// `None` when the line ended before the title.
    pub fn title(&self) -> Option<String> {
        string(&self.0)
    }

    /// A second line of the title as printed, from the `subtitle` field.
    pub fn subtitle(&self) -> Option<StringField> {
        field(&self.0, Keyword::Subtitle)
    }

    /// The official ballot text, copied verbatim, from the `description` field.
    pub fn description(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Description)
    }

    /// The official text of what a yes vote does, copied verbatim, from the `if_yes` field.
    ///
    /// `if_yes` and `if_no` are expected together (`unpaired-outcome`).
    pub fn if_yes(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::IfYes)
    }

    /// The official text of what a no vote does, copied verbatim, from the `if_no` field.
    ///
    /// `if_yes` and `if_no` are expected together (`unpaired-outcome`).
    pub fn if_no(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::IfNo)
    }

    /// How the measure reached the ballot, such as a legislative referral, from the `placement`
    /// field.
    pub fn placement(&self) -> Option<StringField> {
        field(&self.0, Keyword::Placement)
    }

    /// The threshold for passage, in prose, from the `tabulation` field.
    pub fn tabulation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Tabulation)
    }

    /// The official fiscal estimate, in prose, from the `fiscal_impact` field.
    pub fn fiscal_impact(&self) -> Option<StringField> {
        field(&self.0, Keyword::FiscalImpact)
    }

    /// The `supporters` field.
    ///
    /// `None` when the field is absent, which means the author did not research supporters. A
    /// present field with the value `none` means researched and none known; see
    /// [`StringListField::value`].
    pub fn supporters(&self) -> Option<StringListField> {
        field(&self.0, Keyword::Supporters)
    }

    /// The `opponents` field.
    ///
    /// `None` when the field is absent, which means the author did not research opponents. A
    /// present field with the value `none` means researched and none known; see
    /// [`StringListField::value`].
    pub fn opponents(&self) -> Option<StringListField> {
        field(&self.0, Keyword::Opponents)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// Every `recommend` in the block, in source order.
    ///
    /// A valid measure has exactly one; this returns them all so that validation can report a
    /// second one where it is written.
    pub fn recommendations(&self) -> impl Iterator<Item = MeasureRecommend> + use<> {
        block_children(&self.0)
    }
}

impl MeasureRecommend {
    /// The position taken.
    ///
    /// `None` when the line ended before the position, which the parser reports as
    /// `missing-value`.
    pub fn position(&self) -> Option<Position> {
        tokens(&self.0).find_map(|t| match t.kind() {
            SyntaxKind::YesKeyword => Some(Position::Yes),
            SyntaxKind::NoKeyword => Some(Position::No),
            SyntaxKind::NoPositionKeyword => Some(Position::NoPosition),
            _ => None,
        })
    }

    /// Why the author takes this position, from the `justification` field.
    pub fn justification(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Justification)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }
}

impl StringListField {
    /// The keyword that names the field, [`Keyword::Supporters`] or [`Keyword::Opponents`].
    ///
    /// The parser begins every field with its keyword, so this is `None` only for a node built
    /// some other way.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The strings, with escapes resolved.
    ///
    /// `Some` of an empty list for `none`. `None` when the line ended before the value or the
    /// value is neither `none` nor a string, which the parser reports.
    pub fn value(&self) -> Option<Vec<String>> {
        if has_token(&self.0, SyntaxKind::NoneValueKeyword) {
            return Some(Vec::new());
        }
        let strings: Vec<String> = tokens(&self.0)
            .filter(|t| t.kind() == SyntaxKind::StringLiteral)
            .map(|t| string_value(t.text()))
            .collect();
        (!strings.is_empty()).then_some(strings)
    }
}
