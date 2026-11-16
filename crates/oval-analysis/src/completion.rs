use oval_core::{Entity, Symbols};
use oval_syntax::ast::{AstNode, Identifier};
use oval_syntax::{
    Expected, Keyword, ParseResult, SyntaxErrorKind, TextRange, TextSize, TokenKind,
};

use crate::Analysis;
use crate::hover::sentence;
use crate::symbols::nameable;

const PLACEHOLDER: &str = "zqovalcompletionzq";

/// The kind of thing a [`Completion`] inserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    /// A keyword, such as `contest` or `single`.
    Keyword,
    /// An identifier, which names something declared as this kind of entity.
    Identifier(Entity),
}

/// Something the grammar accepts at a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// The text to insert, which is a keyword or an identifier with its `@`.
    pub label: String,
    /// What the label is.
    pub kind: CompletionKind,
    /// What a keyword does, or the name an identifier declares.
    pub detail: Option<String>,
    /// The text the label replaces, which is the part of the word written before the position.
    pub replace: TextRange,
}

impl Analysis {
    /// Returns everything the grammar accepts at `offset`, keywords and identifiers alike.
    ///
    /// The word before `offset` is treated as unfinished, and the rest of the line as not yet
    /// written, so the completions are what could begin there. An editor filters them by the text
    /// in [`replace`](Completion::replace).
    ///
    /// A keyword is offered where the parser would accept it, leaving out a field the block already
    /// has and an entry that can't come that late. An identifier is offered where a reference may
    /// name it, so a `party` field offers parties and a `support` line offers the members of the
    /// contest's campaigns. There's nothing to complete inside a string, a comment, or a Markdown
    /// body, or where an identifier is being declared.
    pub fn completions(&self, offset: TextSize) -> Vec<Completion> {
        let source = self.report().parse().syntax().to_string();
        let Some(probe) = Probe::new(&source, offset) else {
            return Vec::new();
        };
        if probe.partial().starts_with('@') {
            return probe.identifiers();
        }
        let mut completions = Vec::new();
        for expected in probe.expected() {
            match expected {
                Expected::Keyword(keyword) if probe.accepts(keyword) => {
                    completions.push(Completion {
                        label: keyword.text().to_owned(),
                        kind: CompletionKind::Keyword,
                        detail: Some(sentence(keyword).to_owned()),
                        replace: probe.replace,
                    });
                }
                Expected::Token(TokenKind::AtSign) => completions.extend(probe.identifiers()),
                _ => {}
            }
        }
        completions
    }
}

struct Probe<'a> {
    source: &'a str,
    replace: TextRange,
    line_end: usize,
}

impl<'a> Probe<'a> {
    fn new(source: &'a str, offset: TextSize) -> Option<Self> {
        let offset = usize::from(offset);
        if !source.is_char_boundary(offset) {
            return None;
        }
        let bytes = source.as_bytes();
        let mut start = offset;
        while start > 0 && is_word_byte(bytes[start - 1]) {
            start -= 1;
        }
        if start > 0 && bytes[start - 1] == b'@' {
            start -= 1;
        }
        let line_end = source[offset..]
            .find(['\n', '\r'])
            .map_or(source.len(), |end| offset + end);
        Some(Self {
            source,
            replace: TextRange::new(text_size(start)?, text_size(offset)?),
            line_end,
        })
    }

    fn partial(&self) -> &str {
        &self.source[self.replace]
    }

    fn start(&self) -> u32 {
        self.replace.start().into()
    }

    fn with(&self, text: &str) -> Option<ParseResult> {
        let start = usize::from(self.replace.start());
        let probed = format!(
            "{}{text}{}",
            &self.source[..start],
            &self.source[self.line_end..]
        );
        oval_syntax::parse(&probed).ok()
    }

    fn expected(&self) -> Vec<Expected> {
        let Some(parse) = self.with(PLACEHOLDER) else {
            return Vec::new();
        };
        parse
            .syntax_errors
            .into_iter()
            .find(|error| {
                error.offset == self.start()
                    && matches!(
                        error.kind,
                        SyntaxErrorKind::UnexpectedToken | SyntaxErrorKind::UnexpectedEntry
                    )
            })
            .map(|error| error.expected)
            .unwrap_or_default()
    }

    fn accepts(&self, keyword: Keyword) -> bool {
        let Some(parse) = self.with(keyword.text()) else {
            return false;
        };
        !parse.syntax_errors.iter().any(|error| {
            error.offset == self.start()
                && matches!(
                    error.kind,
                    SyntaxErrorKind::RepeatedField
                        | SyntaxErrorKind::RepeatedModifier
                        | SyntaxErrorKind::RepeatedFlag
                        | SyntaxErrorKind::MisplacedEntry
                        | SyntaxErrorKind::UnexpectedStance
                )
        })
    }

    fn identifiers(&self) -> Vec<Completion> {
        let Some(parse) = self.with(&format!("@{PLACEHOLDER}")) else {
            return Vec::new();
        };
        let symbols = Symbols::new(&parse.document());
        let Some(reference) = symbols
            .references()
            .iter()
            .find(|reference| is_placeholder(&reference.identifier))
        else {
            return Vec::new();
        };
        nameable(&symbols, reference)
            .into_iter()
            .map(|declaration| Completion {
                label: declaration.identifier.syntax().text().to_string(),
                kind: CompletionKind::Identifier(declaration.entity),
                detail: declaration.name().map(|name| name.value().into_owned()),
                replace: self.replace,
            })
            .collect()
    }
}

fn is_placeholder(identifier: &Identifier) -> bool {
    identifier
        .name()
        .is_some_and(|name| name.text() == PLACEHOLDER)
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
}

fn text_size(offset: usize) -> Option<TextSize> {
    TextSize::try_from(offset).ok()
}

#[cfg(test)]
mod tests {
    use oval_core::Entity;
    use oval_syntax::{TextRange, TextSize};

    use super::{Completion, CompletionKind};
    use crate::Analysis;

    fn completions(marked: &str) -> Vec<Completion> {
        let offset = marked.find('|').unwrap();
        let source = marked.replacen('|', "", 1);
        Analysis::new(&source)
            .unwrap()
            .completions(TextSize::try_from(offset).unwrap())
    }

    fn labels(marked: &str) -> Vec<String> {
        completions(marked)
            .into_iter()
            .map(|completion| completion.label)
            .collect()
    }

    const CONTEST: &str = "oval 1\nparty @dem \"Democratic\"\nparty @grn \"Green\"\ncandidate @n \"Nadia\"\ncandidate @x \"Xavier\"\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @n\n    campaign party @dem\n";

    #[test]
    fn keywords_are_the_entries_a_block_can_still_take() {
        assert_eq!(
            labels("oval 1\nguide \"G\"\n|\n"),
            ["election", "date", "note", "party", "candidate", "ballot"]
        );
        assert_eq!(
            labels("oval 1\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n|\n"),
            ["ballot"]
        );
        assert_eq!(
            labels("oval 1\ncandidate @c \"C\"\n  party @p \"P\"\n  |\n"),
            ["designation", "note"]
        );
    }

    #[test]
    fn keywords_are_what_the_line_can_take_next() {
        assert_eq!(
            labels("oval 1\nballot @b \"B\"\n  measure @m \"M\"\n    recommend |\n"),
            ["yes", "no", "no-position", "abstain"]
        );
        assert_eq!(
            labels(
                "oval 1\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign withdrawn |\n"
            ),
            ["incumbent", "party"]
        );
        assert_eq!(
            labels("oval 1\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @a \"A\" |\n"),
            ["as"]
        );
        assert_eq!(
            labels(
                "oval 1\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @a \"A\"\n    recommend no-position\n      |\n"
            ),
            ["justification", "note"]
        );
    }

    #[test]
    fn an_error_before_the_position_does_not_decide_what_comes_next() {
        assert_eq!(
            labels("oval 1\nballot @b \"B\"\n  contest @k \"K\" bogus |\n"),
            [""; 0]
        );
        assert_eq!(
            labels("oval 1\nballot @b \"B\"\n  bogus\n  measure @m \"M\"\n    recommend yes\n|\n"),
            ["ballot"]
        );
    }

    #[test]
    fn a_keyword_replaces_the_word_before_the_position_and_says_what_it_does() {
        let marked = "oval 1\nballot @b \"B\"\n  contest @k \"K\" sin|gle\n";
        let completions = completions(marked);
        let single = &completions[0];
        assert_eq!(single.label, "single");
        assert_eq!(single.kind, CompletionKind::Keyword);
        assert_eq!(
            single.detail.as_deref(),
            Some("makes the contest a vote for one choice")
        );
        let start = TextSize::try_from(marked.find("sin").unwrap()).unwrap();
        assert_eq!(single.replace, TextRange::at(start, TextSize::of("sin")));
        assert_eq!(
            labels("oval 1\nba|"),
            [
                "guide",
                "election",
                "date",
                "note",
                "party",
                "candidate",
                "ballot"
            ]
        );
    }

    #[test]
    fn a_party_field_offers_each_party_once_with_its_name() {
        let marked = "oval 1\nparty @dem \"Democratic\"\nparty @dem \"Again\"\ncandidate @n \"N\"\n  party @d|\n";
        let completions = completions(marked);
        assert_eq!(completions.len(), 1);
        let dem = &completions[0];
        assert_eq!(dem.label, "@dem");
        assert_eq!(dem.kind, CompletionKind::Identifier(Entity::Party));
        assert_eq!(dem.detail.as_deref(), Some("Democratic"));
        let start = TextSize::try_from(marked.rfind("@d").unwrap()).unwrap();
        assert_eq!(dem.replace, TextRange::at(start, TextSize::of("@d")));
    }

    #[test]
    fn a_campaign_offers_candidates_and_a_party_campaign_offers_parties() {
        let campaign = format!("{CONTEST}    campaign |\n");
        assert_eq!(
            labels(&campaign),
            ["incumbent", "withdrawn", "party", "@n", "@x"]
        );
        let party = format!("{CONTEST}    campaign party @|\n");
        assert_eq!(labels(&party), ["@dem", "@grn"]);
    }

    #[test]
    fn a_stance_offers_the_members_of_the_contests_campaigns() {
        let support = format!("{CONTEST}    recommend\n      support |\n");
        assert_eq!(labels(&support), ["@n", "@dem", "write-in"]);
        let oppose = format!("{CONTEST}    recommend\n      oppose @|\n");
        let completions = completions(&oppose);
        assert_eq!(
            completions
                .iter()
                .map(|completion| (completion.label.as_str(), completion.kind))
                .collect::<Vec<_>>(),
            [
                ("@n", CompletionKind::Identifier(Entity::Candidate)),
                ("@dem", CompletionKind::Identifier(Entity::Party)),
            ]
        );
    }

    #[test]
    fn nothing_is_offered_inside_text_or_where_an_identifier_is_declared() {
        assert_eq!(labels("oval 1\nguide \"G|\"\n"), [""; 0]);
        assert_eq!(labels("oval 1\nguide \"G\" # a |comment\n"), [""; 0]);
        assert_eq!(
            labels(
                "oval 1\nballot @b \"B\"\n  measure @m \"M\"\n    note:\n      some |text\n    recommend yes\n"
            ),
            [""; 0]
        );
        assert_eq!(labels("oval 1\nballot @|\n"), [""; 0]);
        assert_eq!(labels("oval 1\nguide \"é|\"\n"), [""; 0]);
    }
}
