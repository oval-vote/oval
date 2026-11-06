macro_rules! define_syntax {
    (
        keywords { $( $keyword_text:literal => $keyword:ident / $keyword_kind:ident ),+ $(,)? }
        tokens { $( $token:ident : $token_doc:literal ),+ $(,)? }
        nodes { $( $node:ident : $node_doc:literal ),+ $(,)? }
    ) => {
        /// A reserved word of the Oval language.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum Keyword {
            $( #[doc = concat!("The `", $keyword_text, "` keyword.")] $keyword, )+
        }

        impl Keyword {
            /// Every keyword, in declaration order.
            pub const ALL: &'static [Self] = &[ $( Self::$keyword, )+ ];

            /// The source text of this keyword.
            pub const fn text(self) -> &'static str {
                match self { $( Self::$keyword => $keyword_text, )+ }
            }

            /// The keyword whose source text is `text`, if any.
            pub fn from_text(text: &str) -> Option<Self> {
                match text { $( $keyword_text => Some(Self::$keyword), )+ _ => None }
            }
        }

        /// Every kind of token and node in an Oval syntax tree.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        #[non_exhaustive]
        pub enum SyntaxKind {
            $( #[doc = concat!("The `", $keyword_text, "` keyword.")] $keyword_kind, )+
            $( #[doc = $token_doc] $token, )+
            $( #[doc = $node_doc] $node, )+
            #[doc(hidden)]
            __Last,
        }

        const ALL: &[SyntaxKind] = &[
            $( SyntaxKind::$keyword_kind, )+
            $( SyntaxKind::$token, )+
            $( SyntaxKind::$node, )+
            SyntaxKind::__Last,
        ];

        impl SyntaxKind {
            /// The keyword this kind represents, if it is a keyword token.
            pub const fn as_keyword(self) -> Option<Keyword> {
                match self {
                    $( Self::$keyword_kind => Some(Keyword::$keyword), )+
                    _ => None,
                }
            }

            /// Whether this kind is a node rather than a token.
            pub const fn is_node(self) -> bool {
                matches!(self, $( Self::$node )|+)
            }
        }

        impl From<Keyword> for SyntaxKind {
            fn from(keyword: Keyword) -> Self {
                match keyword { $( Keyword::$keyword => Self::$keyword_kind, )+ }
            }
        }
    };
}

define_syntax! {
    keywords {
        "oval" => Oval / OvalKeyword,
        "guide" => Guide / GuideKeyword,
        "election" => Election / ElectionKeyword,
        "date" => Date / DateKeyword,
        "note" => Note / NoteKeyword,
        "party" => Party / PartyKeyword,
        "candidate" => Candidate / CandidateKeyword,
        "ballot" => Ballot / BallotKeyword,
        "section" => Section / SectionKeyword,
        "contest" => Contest / ContestKeyword,
        "campaign" => Campaign / CampaignKeyword,
        "measure" => Measure / MeasureKeyword,
        "abbreviation" => Abbreviation / AbbreviationKeyword,
        "website" => Website / WebsiteKeyword,
        "designation" => Designation / DesignationKeyword,
        "eligibility" => Eligibility / EligibilityKeyword,
        "single" => Single / SingleKeyword,
        "multiple" => Multiple / MultipleKeyword,
        "ranked" => Ranked / RankedKeyword,
        "retention" => Retention / RetentionKeyword,
        "max" => Max / MaxKeyword,
        "subtitle" => Subtitle / SubtitleKeyword,
        "tabulation" => Tabulation / TabulationKeyword,
        "incumbent" => Incumbent / IncumbentKeyword,
        "withdrawn" => Withdrawn / WithdrawnKeyword,
        "as" => As / AsKeyword,
        "recommend" => Recommend / RecommendKeyword,
        "support" => Support / SupportKeyword,
        "oppose" => Oppose / OpposeKeyword,
        "justification" => Justification / JustificationKeyword,
        "yes" => Yes / YesKeyword,
        "no" => No / NoKeyword,
        "no-position" => NoPosition / NoPositionKeyword,
        "description" => Description / DescriptionKeyword,
        "if_yes" => IfYes / IfYesKeyword,
        "if_no" => IfNo / IfNoKeyword,
        "placement" => Placement / PlacementKeyword,
        "fiscal_impact" => FiscalImpact / FiscalImpactKeyword,
        "supporters" => Supporters / SupportersKeyword,
        "opponents" => Opponents / OpponentsKeyword,
        "none" => NoneValue / NoneValueKeyword,
    }
    tokens {
        Identifier: "An identifier token: `@` followed by a name.",
        StringLiteral: "A double-quoted string token.",
        IntegerLiteral: "An unsigned integer token.",
        DateLiteral: "An ISO 8601 calendar date token.",
        UrlLiteral: "A bare `http` or `https` URL token.",
        Colon: "The `:` punctuator.",
        Comma: "The `,` punctuator.",
        MarkdownBlock: "The body of a Markdown field, as one opaque token.",
        Comment: "A `#` comment running to the end of the line.",
        Whitespace: "Spaces within a line.",
        Newline: "The end of a non-blank line.",
        BlankLine: "The end of a line with no tokens.",
        Indent: "An increase in indentation.",
        Dedent: "A decrease in indentation.",
        Error: "Text the lexer could not tokenize.",
    }
    nodes {
        Document: "The root node: a whole document.",
        VersionMarker: "The `oval` version marker.",
        Party: "A `party` declaration.",
        Candidate: "A `candidate` declaration.",
        PartyReference: "A `party` field on a candidate or campaign.",
        Ballot: "A `ballot` declaration.",
        Section: "A `section`.",
        Contest: "A `contest` declaration.",
        ContestKind: "The voting method on a contest header line.",
        Campaign: "A `campaign`.",
        Member: "A member of a campaign.",
        ContestRecommend: "A contest `recommend`.",
        Support: "A `support` tier.",
        Oppose: "An `oppose` line.",
        Measure: "A `measure` declaration.",
        MeasureRecommend: "A measure `recommend`.",
        Block: "An indented block of fields and children.",
        StringField: "A field whose value is a string.",
        IntegerField: "A field whose value is an integer.",
        DateField: "A field whose value is a date.",
        UrlField: "A field whose value is a URL.",
        ReferenceField: "A field whose value is an identifier reference.",
        MarkdownField: "A field whose value is a Markdown block.",
        StringListField: "A field whose value is a list of strings or `none`.",
        Flag: "A bare keyword flag on a header line.",
        ErrorNode: "A node the parser could not recognize.",
    }
}

impl SyntaxKind {
    /// Whether this kind is a keyword token.
    pub const fn is_keyword(self) -> bool {
        self.as_keyword().is_some()
    }

    /// Whether this kind is a token rather than a node.
    pub const fn is_token(self) -> bool {
        !self.is_node() && !matches!(self, Self::__Last)
    }

    /// Whether this kind is a token the parser ignores.
    pub const fn is_trivia(self) -> bool {
        matches!(self, Self::Comment | Self::Whitespace | Self::BlankLine)
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

impl From<rowan::SyntaxKind> for SyntaxKind {
    fn from(raw: rowan::SyntaxKind) -> Self {
        assert!(raw.0 < Self::__Last as u16, "invalid SyntaxKind {}", raw.0);
        ALL[usize::from(raw.0)]
    }
}

#[cfg(test)]
mod tests {
    use super::{ALL, Keyword, SyntaxKind};

    #[test]
    fn all_is_in_declaration_order() {
        for (i, kind) in ALL.iter().enumerate() {
            assert_eq!(*kind as usize, i, "ALL[{i}] is {kind:?}");
        }
        assert_eq!(ALL.len(), SyntaxKind::__Last as usize + 1);
    }

    #[test]
    fn raw_round_trips_for_every_kind() {
        for kind in ALL.iter().copied().filter(|k| *k != SyntaxKind::__Last) {
            let raw: rowan::SyntaxKind = kind.into();
            assert_eq!(SyntaxKind::from(raw), kind);
        }
    }

    #[test]
    fn keywords_round_trip_through_text() {
        assert_eq!(Keyword::ALL.len(), 41, "Appendix B lists 41 keywords");
        for keyword in Keyword::ALL.iter().copied() {
            assert_eq!(
                Keyword::from_text(keyword.text()),
                Some(keyword),
                "{keyword:?}"
            );
        }
    }

    #[test]
    fn keywords_round_trip_through_syntax_kind() {
        for keyword in Keyword::ALL.iter().copied() {
            let kind = SyntaxKind::from(keyword);
            assert!(kind.is_keyword(), "{kind:?}");
            assert_eq!(kind.as_keyword(), Some(keyword), "{kind:?}");
        }
    }

    #[test]
    fn keywords_are_case_sensitive_and_closed() {
        assert_eq!(Keyword::from_text("Guide"), None);
        assert_eq!(Keyword::from_text("no_position"), None);
        assert_eq!(Keyword::from_text(""), None);
        assert_eq!(SyntaxKind::Identifier.as_keyword(), None);
    }

    #[test]
    fn classification_is_a_partition() {
        for kind in ALL.iter().copied().filter(|k| *k != SyntaxKind::__Last) {
            assert_ne!(kind.is_token(), kind.is_node(), "{kind:?}");
            assert!(!kind.is_keyword() || kind.is_token(), "{kind:?}");
            assert!(!kind.is_trivia() || kind.is_token(), "{kind:?}");
        }
        assert!(SyntaxKind::Comment.is_trivia());
        assert!(SyntaxKind::BlankLine.is_trivia());
        assert!(!SyntaxKind::Newline.is_trivia());
    }
}
