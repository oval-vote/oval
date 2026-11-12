/// A word that the Oval grammar names literally, such as `ballot` or `if_yes`.
///
/// The keywords are the ones Appendix B of the specification lists. The lexer does not know them, so
/// a keyword is a [`TokenKind::Word`](crate::TokenKind::Word) token like any other word, and a word
/// is a keyword only where the grammar expects one. Keywords are case-sensitive, so `Ballot` is not
/// a keyword. None is reserved, since every name an author chooses follows an `@`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Keyword {
    /// The `oval` keyword.
    Oval,
    /// The `guide` keyword.
    Guide,
    /// The `election` keyword.
    Election,
    /// The `date` keyword.
    Date,
    /// The `note` keyword.
    Note,
    /// The `party` keyword.
    Party,
    /// The `candidate` keyword.
    Candidate,
    /// The `ballot` keyword.
    Ballot,
    /// The `section` keyword.
    Section,
    /// The `contest` keyword.
    Contest,
    /// The `campaign` keyword.
    Campaign,
    /// The `measure` keyword.
    Measure,
    /// The `abbreviation` keyword.
    Abbreviation,
    /// The `website` keyword.
    Website,
    /// The `designation` keyword.
    Designation,
    /// The `eligibility` keyword.
    Eligibility,
    /// The `single` keyword.
    Single,
    /// The `multiple` keyword.
    Multiple,
    /// The `ranked` keyword.
    Ranked,
    /// The `retention` keyword.
    Retention,
    /// The `seats` keyword.
    Seats,
    /// The `max` keyword.
    Max,
    /// The `min` keyword.
    Min,
    /// The `all` keyword.
    All,
    /// The `subtitle` keyword.
    Subtitle,
    /// The `tabulation` keyword.
    Tabulation,
    /// The `incumbent` keyword.
    Incumbent,
    /// The `withdrawn` keyword.
    Withdrawn,
    /// The `as` keyword.
    As,
    /// The `required` keyword.
    Required,
    /// The `optional` keyword.
    Optional,
    /// The `extensible` keyword.
    Extensible,
    /// The `exhaustive` keyword.
    Exhaustive,
    /// The `recommend` keyword.
    Recommend,
    /// The `support` keyword.
    Support,
    /// The `oppose` keyword.
    Oppose,
    /// The `justification` keyword.
    Justification,
    /// The `yes` keyword.
    Yes,
    /// The `no` keyword.
    No,
    /// The `no-position` keyword.
    NoPosition,
    /// The `abstain` keyword.
    Abstain,
    /// The `write-in` keyword.
    WriteIn,
    /// The `description` keyword.
    Description,
    /// The `if_yes` keyword.
    IfYes,
    /// The `if_no` keyword.
    IfNo,
    /// The `placement` keyword.
    Placement,
    /// The `fiscal_impact` keyword.
    FiscalImpact,
    /// The `supporters` keyword.
    Supporters,
    /// The `opponents` keyword.
    Opponents,
    /// The `none` keyword.
    NoneValue,
}

impl Keyword {
    /// Every keyword, in the order Appendix B of the specification lists them.
    pub const ALL: [Self; Keyword::NoneValue as usize + 1] = [
        Self::Oval,
        Self::Guide,
        Self::Election,
        Self::Date,
        Self::Note,
        Self::Party,
        Self::Candidate,
        Self::Ballot,
        Self::Section,
        Self::Contest,
        Self::Campaign,
        Self::Measure,
        Self::Abbreviation,
        Self::Website,
        Self::Designation,
        Self::Eligibility,
        Self::Single,
        Self::Multiple,
        Self::Ranked,
        Self::Retention,
        Self::Seats,
        Self::Max,
        Self::Min,
        Self::All,
        Self::Subtitle,
        Self::Tabulation,
        Self::Incumbent,
        Self::Withdrawn,
        Self::As,
        Self::Required,
        Self::Optional,
        Self::Extensible,
        Self::Exhaustive,
        Self::Recommend,
        Self::Support,
        Self::Oppose,
        Self::Justification,
        Self::Yes,
        Self::No,
        Self::NoPosition,
        Self::Abstain,
        Self::WriteIn,
        Self::Description,
        Self::IfYes,
        Self::IfNo,
        Self::Placement,
        Self::FiscalImpact,
        Self::Supporters,
        Self::Opponents,
        Self::NoneValue,
    ];

    /// Returns the keyword's text, such as `"if_yes"` for [`IfYes`](Self::IfYes).
    pub fn text(self) -> &'static str {
        match self {
            Self::Oval => "oval",
            Self::Guide => "guide",
            Self::Election => "election",
            Self::Date => "date",
            Self::Note => "note",
            Self::Party => "party",
            Self::Candidate => "candidate",
            Self::Ballot => "ballot",
            Self::Section => "section",
            Self::Contest => "contest",
            Self::Campaign => "campaign",
            Self::Measure => "measure",
            Self::Abbreviation => "abbreviation",
            Self::Website => "website",
            Self::Designation => "designation",
            Self::Eligibility => "eligibility",
            Self::Single => "single",
            Self::Multiple => "multiple",
            Self::Ranked => "ranked",
            Self::Retention => "retention",
            Self::Seats => "seats",
            Self::Max => "max",
            Self::Min => "min",
            Self::All => "all",
            Self::Subtitle => "subtitle",
            Self::Tabulation => "tabulation",
            Self::Incumbent => "incumbent",
            Self::Withdrawn => "withdrawn",
            Self::As => "as",
            Self::Required => "required",
            Self::Optional => "optional",
            Self::Extensible => "extensible",
            Self::Exhaustive => "exhaustive",
            Self::Recommend => "recommend",
            Self::Support => "support",
            Self::Oppose => "oppose",
            Self::Justification => "justification",
            Self::Yes => "yes",
            Self::No => "no",
            Self::NoPosition => "no-position",
            Self::Abstain => "abstain",
            Self::WriteIn => "write-in",
            Self::Description => "description",
            Self::IfYes => "if_yes",
            Self::IfNo => "if_no",
            Self::Placement => "placement",
            Self::FiscalImpact => "fiscal_impact",
            Self::Supporters => "supporters",
            Self::Opponents => "opponents",
            Self::NoneValue => "none",
        }
    }

    /// Returns the keyword whose text is exactly `text`, or `None` if `text` is not a keyword.
    ///
    /// # Examples
    ///
    /// ```
    /// use oval_syntax::Keyword;
    ///
    /// assert_eq!(Keyword::from_text("if_yes"), Some(Keyword::IfYes));
    /// assert_eq!(Keyword::from_text("Ballot"), None);
    /// ```
    pub fn from_text(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|keyword| keyword.text() == text)
    }
}

const _: () = {
    let mut i = 0;
    while i < Keyword::ALL.len() {
        assert!(
            Keyword::ALL[i] as usize == i,
            "Keyword::ALL lists every keyword in order"
        );
        i += 1;
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TokenKind, lex};

    #[test]
    fn every_keyword_is_found_by_its_text() {
        for keyword in Keyword::ALL {
            assert_eq!(
                Keyword::from_text(keyword.text()),
                Some(keyword),
                "{keyword:?}"
            );
        }
    }

    #[test]
    fn every_keyword_lexes_as_one_word() {
        for keyword in Keyword::ALL {
            let kinds: Vec<TokenKind> = lex(keyword.text())
                .unwrap()
                .tokens
                .iter()
                .map(|token| token.kind)
                .collect();
            assert_eq!(kinds, [TokenKind::Word, TokenKind::Newline], "{keyword:?}");
        }
    }

    #[test]
    fn keywords_with_hyphens_and_underscores_keep_them() {
        assert_eq!(Keyword::NoPosition.text(), "no-position");
        assert_eq!(Keyword::WriteIn.text(), "write-in");
        assert_eq!(Keyword::IfYes.text(), "if_yes");
        assert_eq!(Keyword::FiscalImpact.text(), "fiscal_impact");
    }

    #[test]
    fn the_none_value_is_the_word_none() {
        assert_eq!(Keyword::from_text("none"), Some(Keyword::NoneValue));
    }

    #[test]
    fn other_words_are_not_keywords() {
        for text in ["Ballot", "ballots", "if-yes", "", "@oval"] {
            assert_eq!(Keyword::from_text(text), None, "{text:?}");
        }
    }
}
