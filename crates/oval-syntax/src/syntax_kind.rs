//! The kinds of token and node in an Oval syntax tree, and the keywords of the language.
//!
//! [`Keyword`] lists the words of Appendix B of the specification. [`SyntaxKind`] gives every
//! keyword a token kind, adds the other tokens of Sections 2.2 through 2.4 and the nodes the
//! parser builds, and is the kind type of the [`rowan`] tree. Both are re-exported at the crate
//! root.

macro_rules! define_syntax {
    (
        keywords { $( $keyword_text:literal => $keyword:ident / $keyword_kind:ident ),+ $(,)? }
        tokens { $( $token:ident : $token_doc:literal ),+ $(,)? }
        nodes { $( $node:ident : $node_doc:literal ),+ $(,)? }
    ) => {
        /// A word that a syntactic production of the Oval language names literally.
        ///
        /// Appendix B of the specification lists the keywords. They are case-sensitive, and no
        /// word is reserved, since every identifier begins with `@`. The lexer does not
        /// distinguish them. Every bare word lexes as [`SyntaxKind::Word`], and the parser
        /// classifies words with [`from_text`](Self::from_text). Each keyword has a token kind of
        /// its own in [`SyntaxKind`], reached through [`From`] and [`SyntaxKind::as_keyword`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum Keyword {
            $( #[doc = concat!("The `", $keyword_text, "` keyword.")] $keyword, )+
        }

        impl Keyword {
            /// Every keyword, in the order Appendix B lists them.
            pub const ALL: &'static [Self] = &[ $( Self::$keyword, )+ ];

            /// The source text of this keyword, such as `ballot`.
            pub const fn text(self) -> &'static str {
                match self { $( Self::$keyword => $keyword_text, )+ }
            }

            /// The keyword whose source text is exactly `text`, or `None` for any other word.
            ///
            /// The match is case-sensitive, so `Ballot` is not a keyword.
            ///
            /// # Examples
            ///
            /// ```
            /// use oval_syntax::Keyword;
            ///
            /// assert_eq!(Keyword::from_text("ballot"), Some(Keyword::Ballot));
            /// assert_eq!(Keyword::from_text("Ballot"), None);
            /// assert_eq!(Keyword::Ballot.text(), "ballot");
            /// ```
            pub fn from_text(text: &str) -> Option<Self> {
                match text { $( $keyword_text => Some(Self::$keyword), )+ _ => None }
            }
        }

        /// Every kind of token and node in an Oval syntax tree.
        ///
        /// The kinds come in three groups.
        ///
        /// - One token kind per [`Keyword`].
        /// - The other token kinds of Sections 2.2 through 2.4 of the specification.
        /// - The node kinds the parser builds.
        ///
        /// [`is_token`](Self::is_token) and [`is_node`](Self::is_node) partition the kinds;
        /// [`is_keyword`](Self::is_keyword) and [`is_trivia`](Self::is_trivia) pick out subsets
        /// of the tokens. The enum is non-exhaustive because later editions of the language add
        /// constructs.
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
            /// The keyword this kind is the token of, or `None` for any other kind.
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
        Word: "A bare word, which is a letter followed by letters, digits, hyphens, and underscores. The lexer emits it for every word; in the tree, a word whose text is a keyword has the keyword's kind unless it follows `@`.",
        At: "The `@` punctuator, which begins an identifier.",
        StringLiteral: "A double-quoted string, with its quotes and escapes as written.",
        IntegerLiteral: "An unsigned integer, leading zeros included.",
        DateLiteral: "A calendar date in the shape `YYYY-MM-DD`; whether it exists on the calendar is a validation rule.",
        UrlLiteral: "A bare `http` or `https` URL, running to the next space, tab, or line terminator.",
        Colon: "The `:` punctuator, which opens a Markdown body.",
        Comma: "The `,` punctuator, which separates the strings of a list.",
        MarkdownBlock: "The body of a Markdown field as one opaque token, from the first content line's leading spaces through the last content line's terminator.",
        Comment: "A `#` comment running to the end of the line.",
        Whitespace: "A run of spaces within a line, or a byte order mark at the start of the input.",
        Newline: "The terminator of a line with tokens; zero-length at end of input.",
        BlankLine: "The terminator of a line with no tokens other than whitespace and a comment.",
        Indent: "A zero-length token before the first token of a line indented more than the enclosing level.",
        Dedent: "A zero-length token before the first token of a line that returns to an enclosing level, or at end of input, one per level closed.",
        Error: "Text the lexer could not tokenize; exactly one [`LexicalError`](crate::lexer::LexicalError) spans it.",
    }
    nodes {
        Document: "The root node, a whole document whose text is the source.",
        Identifier: "An identifier, which is `@` followed by a word.",
        VersionMarker: "The `oval` version marker that opens a document.",
        Party: "A top-level `party` declaration with its block.",
        Candidate: "A top-level `candidate` declaration with its block.",
        PartyReference: "A `party` field on a candidate or campaign member, which is either a reference or an inline declaration with its name and block.",
        Ballot: "A `ballot` declaration with its block.",
        Section: "A `section` with its block.",
        Contest: "A `contest` declaration with its block.",
        ContestKind: "The voting method on a contest header line, with its `multiple` count or `max` limit.",
        Campaign: "A `campaign` with its block.",
        Member: "A member of a campaign, on the header line or in the block. It is an identifier, optionally with a name and an `as` role.",
        ContestRecommend: "A contest `recommend` with its block.",
        Support: "A `support` line of a contest recommendation, which is one tier of identifiers.",
        Oppose: "An `oppose` line of a contest recommendation.",
        Measure: "A `measure` declaration with its block.",
        MeasureRecommend: "A measure `recommend` with its position and block.",
        Block: "An indented block of fields and children, from `Indent` to `Dedent`.",
        StringField: "A field whose value is a string.",
        IntegerField: "A field whose value is an integer.",
        DateField: "A field whose value is a date.",
        UrlField: "A field whose value is a URL.",
        ReferenceField: "A field whose value is an identifier reference.",
        MarkdownField: "A field whose value is a Markdown body, made of the keyword, the `:`, and the `MarkdownBlock` token.",
        StringListField: "A field whose value is a comma-separated list of strings or `none`.",
        Flag: "An `incumbent` or `withdrawn` flag on a `campaign` line.",
        ErrorNode: "Text the parser discarded or set aside, kept so that the tree covers every byte of the source.",
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

    /// Whether this kind is a comment, whitespace, or a blank line, the ignored tokens of
    /// Section 2.3.
    ///
    /// [`Error`](Self::Error) tokens are not trivia, although the parser skips them too.
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
        assert_eq!(SyntaxKind::Word.as_keyword(), None);
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
