use super::{
    Identifier, MarkdownField, StringField, ast_node, block_children, field, has_token, identifier,
    keyword, string, string_value, tokens,
};
use crate::{Keyword, SyntaxKind};

ast_node! {
    /// A `measure`: a yes-or-no question.
    Measure
}

ast_node! {
    /// A measure's `recommend`.
    MeasureRecommend
}

ast_node! {
    /// A field whose value is a list of strings, or `none`.
    StringListField
}

/// How the author advises voting on a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Position {
    /// Vote yes.
    Yes,
    /// Vote no.
    No,
    /// The author deliberately takes no position.
    NoPosition,
}

impl Measure {
    /// The declared identifier.
    pub fn id(&self) -> Option<Identifier> {
        identifier(&self.0)
    }

    /// The measure as named on the ballot.
    pub fn title(&self) -> Option<String> {
        string(&self.0)
    }

    /// The `subtitle` field.
    pub fn subtitle(&self) -> Option<StringField> {
        field(&self.0, Keyword::Subtitle)
    }

    /// The `description` field: official ballot text.
    pub fn description(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Description)
    }

    /// The `if_yes` field: official ballot text.
    pub fn if_yes(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::IfYes)
    }

    /// The `if_no` field: official ballot text.
    pub fn if_no(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::IfNo)
    }

    /// The `placement` field.
    pub fn placement(&self) -> Option<StringField> {
        field(&self.0, Keyword::Placement)
    }

    /// The `tabulation` field.
    pub fn tabulation(&self) -> Option<StringField> {
        field(&self.0, Keyword::Tabulation)
    }

    /// The `fiscal_impact` field.
    pub fn fiscal_impact(&self) -> Option<StringField> {
        field(&self.0, Keyword::FiscalImpact)
    }

    /// The `supporters` field. Absent means not researched; see [`StringListField::value`].
    pub fn supporters(&self) -> Option<StringListField> {
        field(&self.0, Keyword::Supporters)
    }

    /// The `opponents` field. Absent means not researched; see [`StringListField::value`].
    pub fn opponents(&self) -> Option<StringListField> {
        field(&self.0, Keyword::Opponents)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }

    /// Every `recommend` in the block. A valid measure has exactly one.
    pub fn recommendations(&self) -> impl Iterator<Item = MeasureRecommend> + use<> {
        block_children(&self.0)
    }
}

impl MeasureRecommend {
    /// The position taken, if one was written.
    pub fn position(&self) -> Option<Position> {
        tokens(&self.0).find_map(|t| match t.kind() {
            SyntaxKind::YesKeyword => Some(Position::Yes),
            SyntaxKind::NoKeyword => Some(Position::No),
            SyntaxKind::NoPositionKeyword => Some(Position::NoPosition),
            _ => None,
        })
    }

    /// The `justification` field.
    pub fn justification(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Justification)
    }

    /// The `note` field.
    pub fn note(&self) -> Option<MarkdownField> {
        field(&self.0, Keyword::Note)
    }
}

impl StringListField {
    /// Which field this is.
    pub fn keyword(&self) -> Option<Keyword> {
        keyword(&self.0)
    }

    /// The strings, with escapes resolved. `Some(empty)` for `none`; `None` when the value is
    /// missing or malformed.
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
