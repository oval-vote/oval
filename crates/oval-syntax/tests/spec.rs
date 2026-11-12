//! Checks the lexer, the parser, and the typed views against the specification's examples.
//!
//! TEMPORARY: these tests and their copies of the specification in `fixtures/SPEC.md` and of its
//! example guide in `fixtures/franklin.oval` exist to check the rewrite during development. Delete
//! them before the rewrite is merged. Refresh the copies with
//! `cp ../oval-spec/SPEC.md ../oval-spec/examples/franklin.oval crates/oval-syntax/tests/fixtures/`.

use std::collections::HashSet;

use oval_syntax::ast::{
    AstNode, AstToken, Ballot, BallotItem, Campaign, Candidate, Choice, Contest, ContestKind,
    ContestRecommend, DateField, Document, Identifier, MarkdownField, Measure, MeasureRecommend,
    Member, Party, PartyMember, PartyRef, Section, SectionItem, Stance, StringField,
    StringListField, StringLiteral, UrlField,
};
use oval_syntax::{ParseResult, SyntaxKind, SyntaxNode, SyntaxToken, TokenKind, lex, parse};

const SPEC: &str = include_str!("fixtures/SPEC.md");

const FRANKLIN: &str = include_str!("fixtures/franklin.oval");

const CONTEXTS: &[(&str, &str, usize)] = &[
    ("document", "", 0),
    ("ballot", "ballot @x \"X\"\n", 2),
    ("section", "ballot @x \"X\"\n  section \"S\"\n", 4),
    ("contest", "ballot @x \"X\"\n  contest @y \"Y\" single\n", 4),
    ("measure", "ballot @x \"X\"\n  measure @y \"Y\"\n", 4),
    ("party", "party @x \"X\"\n", 2),
    ("candidate", "candidate @x \"X\"\n", 2),
    (
        "campaign",
        "ballot @x \"X\"\n  contest @y \"Y\" single\n    campaign\n",
        6,
    ),
];
const NOT_YET_LEXED: &[&str] = &[];

const NOT_YET_REPORTED: &[&str] = &[];

struct Block<'a> {
    info: &'a str,
    content: String,
    line: usize,
}

fn blocks(spec: &str) -> Vec<Block<'_>> {
    let mut blocks = Vec::new();
    let mut lines = spec.lines().enumerate();
    while let Some((index, line)) = lines.next() {
        let Some(info) = line.strip_prefix("```") else {
            continue;
        };
        let mut content = String::new();
        for (_, line) in lines.by_ref() {
            if line == "```" {
                break;
            }
            content.push_str(line);
            content.push('\n');
        }
        blocks.push(Block {
            info,
            content,
            line: index + 1,
        });
    }
    blocks
}

fn parse_line(line: &str) -> (&str, Option<String>) {
    match line.split_once(" «") {
        Some((name, rest)) => {
            let text = rest.strip_suffix('»').expect("the text ends with »");
            (name, Some(decode(text)))
        }
        None => (line, None),
    }
}

fn decode(text: &str) -> String {
    text.replace('␣', " ").replace('⇥', "\t").replace('↵', "\n")
}

fn token_kind(name: &str, text: &str) -> TokenKind {
    match (name, text) {
        ("Word", _) => TokenKind::Word,
        ("String", _) => TokenKind::String,
        ("Integer", _) => TokenKind::Integer,
        ("Date", _) => TokenKind::Date,
        ("Url", _) => TokenKind::Url,
        ("Punctuator", "@") => TokenKind::AtSign,
        ("Punctuator", ":") => TokenKind::Colon,
        ("Punctuator", ",") => TokenKind::Comma,
        ("MarkdownBlock", _) => TokenKind::MarkdownBlock,
        ("Whitespace", _) => TokenKind::Whitespace,
        ("Comment", _) => TokenKind::Comment,
        ("BlankLine", _) => TokenKind::BlankLine,
        ("ByteOrderMark", _) => TokenKind::ByteOrderMark,
        ("Newline", _) => TokenKind::Newline,
        ("Indent", _) => TokenKind::Indent,
        ("Dedent", _) => TokenKind::Dedent,
        ("Error", _) => TokenKind::Error,
        _ => panic!("the specification uses an unknown token: {name} «{text}»"),
    }
}

#[test]
fn lexer_matches_the_specification_examples() {
    let blocks = blocks(SPEC);
    let mut checked = 0;
    let mut skipped = Vec::new();
    let mut failures = Vec::new();

    for (index, block) in blocks.iter().enumerate() {
        if block.info != "tokens" {
            continue;
        }
        let errors_block = blocks.get(index + 1).filter(|next| next.info == "errors");
        let uses_tokens_not_lexed = block
            .content
            .lines()
            .any(|line| NOT_YET_LEXED.contains(&parse_line(line).0));
        let expects_errors_not_reported = errors_block.is_some_and(|errors| {
            errors
                .content
                .lines()
                .any(|line| NOT_YET_REPORTED.contains(&parse_line(line).0))
        });
        if uses_tokens_not_lexed || expects_errors_not_reported {
            skipped.push(block.line);
            continue;
        }

        let source = &blocks[..index]
            .iter()
            .rev()
            .find(|block| block.info.starts_with("oval"))
            .expect("an example comes before each token sequence")
            .content;
        let expected_errors: Vec<(String, String)> = errors_block
            .map(|next| {
                next.content
                    .lines()
                    .map(|line| {
                        let (identifier, text) = parse_line(line);
                        (identifier.to_owned(), text.unwrap_or_default())
                    })
                    .collect()
            })
            .unwrap_or_default();
        let expected_tokens: Vec<(TokenKind, String)> = block
            .content
            .lines()
            .map(|line| {
                let (name, text) = parse_line(line);
                let text = text.unwrap_or_default();
                (token_kind(name, &text), text)
            })
            .collect();

        let result = lex(source).expect("the example is accepted");
        let mut pos = 0;
        let actual_tokens: Vec<(TokenKind, String)> = result
            .tokens
            .iter()
            .map(|token| {
                let len = token.len as usize;
                let text = source[pos..pos + len].to_owned();
                pos += len;
                (token.kind, text)
            })
            .collect();
        let actual_errors: Vec<(String, String)> = result
            .errors
            .iter()
            .map(|error| {
                let start = error.offset as usize;
                let text = source[start..start + error.len as usize].to_owned();
                (error.kind.spec_id().to_owned(), text)
            })
            .collect();

        checked += 1;
        if actual_tokens != expected_tokens || actual_errors != expected_errors {
            failures.push(format!(
                "SPEC.md line {}, example:\n{source}\
                 expected tokens: {expected_tokens:?}\n  actual tokens: {actual_tokens:?}\n\
                 expected errors: {expected_errors:?}\n  actual errors: {actual_errors:?}",
                block.line,
            ));
        }
    }

    println!(
        "checked {checked} examples; skipped {} that need work not done yet (lines {skipped:?})",
        skipped.len(),
    );
    assert!(checked > 0, "no specification examples were checked");
    assert!(
        failures.is_empty(),
        "{} of {checked} specification examples failed:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}

fn heading_before(spec: &str, line: usize) -> (&str, &str) {
    let mut section = "";
    let mut heading = "";
    for text in spec.lines().take(line) {
        if let Some(rest) = text.strip_prefix("### ") {
            section = rest;
        } else if let Some(rest) = text.strip_prefix("#### ") {
            heading = rest;
        }
    }
    (section, heading)
}

fn in_context(example: &str, prefix: &str, indent: usize) -> String {
    let mut text = prefix.to_owned();
    for line in example.lines() {
        if !line.is_empty() {
            text.push_str(&" ".repeat(indent));
        }
        text.push_str(line);
        text.push('\n');
    }
    text
}

fn lexical_errors_are_elisions(text: &str, result: &ParseResult) -> bool {
    result.lexical_errors.iter().all(|error| {
        let start = error.offset as usize;
        error.kind.spec_id() == "unexpected-character"
            && &text[start..start + error.len as usize] == "."
    })
}

fn describe(text: &str, result: &ParseResult) -> String {
    let lexical: Vec<_> = result
        .lexical_errors
        .iter()
        .map(ToString::to_string)
        .collect();
    let syntax: Vec<_> = result
        .syntax_errors
        .iter()
        .map(ToString::to_string)
        .collect();
    format!("{text}  lexical errors: {lexical:?}\n  syntax errors: {syntax:?}")
}

#[test]
fn parser_accepts_the_specification_examples() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for block in blocks(SPEC) {
        if block.info != "oval example" {
            continue;
        }
        checked += 1;
        let accepted = CONTEXTS.iter().any(|(_, prefix, indent)| {
            let text = in_context(&block.content, prefix, *indent);
            let result = parse(&text).expect("the example is accepted");
            lexical_errors_are_elisions(&text, &result)
                && result
                    .syntax_errors
                    .iter()
                    .all(|error| error.spec_id() == "missing-block")
        });
        if !accepted {
            let result = parse(&block.content).expect("the example is accepted");
            failures.push(format!(
                "SPEC.md line {}, as a document:\n{}",
                block.line,
                describe(&block.content, &result),
            ));
        }
    }
    println!("parsed {checked} examples");
    assert!(checked > 0, "no specification examples were parsed");
    assert!(
        failures.is_empty(),
        "{} of {checked} specification examples have errors in every context:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}

#[test]
fn parser_reports_the_error_of_each_syntax_counter_example() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for block in blocks(SPEC) {
        let (section, heading) = heading_before(SPEC, block.line);
        if block.info != "oval counter-example" || !section.starts_with("3.2.") {
            continue;
        }
        checked += 1;
        let expected = heading.to_lowercase().replace(' ', "-");
        let reported = CONTEXTS.iter().any(|(_, prefix, indent)| {
            let text = in_context(&block.content, prefix, *indent);
            let result = parse(&text).expect("the counter-example is accepted");
            let ids: Vec<&str> = result
                .syntax_errors
                .iter()
                .map(oval_syntax::SyntaxError::spec_id)
                .filter(|id| *id != "missing-block" || expected == "missing-block")
                .collect();
            let fits_context = if expected == "unexpected-entry" {
                ids.iter().all(|id| *id == expected)
            } else {
                !ids.contains(&"unexpected-entry")
            };
            result.lexical_errors.is_empty() && ids.contains(&expected.as_str()) && fits_context
        });
        if !reported {
            let result = parse(&block.content).expect("the counter-example is accepted");
            failures.push(format!(
                "SPEC.md line {}, expecting {expected}, as a document:\n{}",
                block.line,
                describe(&block.content, &result),
            ));
        }
    }
    println!("checked {checked} syntax counter-examples");
    assert!(checked > 0, "no syntax counter-examples were checked");
    assert!(
        failures.is_empty(),
        "{} of {checked} syntax counter-examples do not report their error:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}

#[test]
fn parser_accepts_the_example_guide() {
    let result = parse(FRANKLIN).expect("the example guide is accepted");
    assert!(
        result.lexical_errors.is_empty() && result.syntax_errors.is_empty(),
        "{}",
        describe("", &result),
    );
}

#[derive(Default)]
struct Reached {
    nodes: HashSet<SyntaxNode>,
    tokens: HashSet<SyntaxToken>,
}

impl Reached {
    fn node(&mut self, view: &impl AstNode) {
        self.nodes.insert(view.syntax().clone());
    }

    fn token(&mut self, view: &impl AstToken) {
        self.tokens.insert(view.syntax().clone());
    }

    fn raw_token(&mut self, token: Option<SyntaxToken>) {
        self.tokens.extend(token);
    }

    fn string(&mut self, string: Option<StringLiteral>) {
        if let Some(string) = string {
            string.value();
            self.token(&string);
        }
    }

    fn identifier(&mut self, identifier: Option<Identifier>) {
        if let Some(identifier) = identifier {
            self.node(&identifier);
            self.raw_token(identifier.name());
        }
    }

    fn string_field(&mut self, field: Option<StringField>) {
        if let Some(field) = field {
            field.keyword();
            self.node(&field);
            self.string(field.value());
        }
    }

    fn date_field(&mut self, field: Option<DateField>) {
        if let Some(field) = field {
            field.keyword();
            self.node(&field);
            if let Some(date) = field.value() {
                let _ = (date.year(), date.month(), date.day());
                self.token(&date);
            }
        }
    }

    fn url_field(&mut self, field: Option<UrlField>) {
        if let Some(field) = field {
            field.keyword();
            self.node(&field);
            if let Some(url) = field.value() {
                self.token(&url);
            }
        }
    }

    fn markdown_field(&mut self, field: Option<MarkdownField>) {
        if let Some(field) = field {
            field.keyword();
            self.node(&field);
            if let Some(body) = field.body() {
                body.value();
                self.token(&body);
            }
        }
    }

    fn string_list_field(&mut self, field: Option<StringListField>) {
        if let Some(field) = field {
            field.keyword();
            field.is_none_value();
            self.node(&field);
            for string in field.strings() {
                self.string(Some(string));
            }
        }
    }

    fn document(&mut self, document: &Document) {
        self.node(document);
        if let Some(marker) = document.version_marker() {
            self.node(&marker);
            if let Some(version) = marker.version() {
                version.value();
                self.token(&version);
            }
        }
        self.string_field(document.guide());
        self.string_field(document.election());
        self.date_field(document.date());
        self.markdown_field(document.note());
        for party in document.parties() {
            self.party(&party);
        }
        for candidate in document.candidates() {
            self.candidate(&candidate);
        }
        for ballot in document.ballots() {
            self.ballot(&ballot);
        }
    }

    fn party(&mut self, party: &Party) {
        self.node(party);
        self.identifier(party.identifier());
        self.string(party.name());
        self.string_field(party.abbreviation());
        self.url_field(party.website());
        self.markdown_field(party.note());
    }

    fn candidate(&mut self, candidate: &Candidate) {
        self.node(candidate);
        self.identifier(candidate.identifier());
        self.string(candidate.name());
        self.string_field(candidate.designation());
        self.party_ref(candidate.party());
        self.markdown_field(candidate.note());
    }

    fn party_ref(&mut self, party: Option<PartyRef>) {
        if let Some(party) = party {
            party.is_declaration();
            self.node(&party);
            self.identifier(party.identifier());
            self.string(party.name());
            self.string_field(party.abbreviation());
            self.url_field(party.website());
            self.markdown_field(party.note());
        }
    }

    fn modifiers(
        &mut self,
        requirement: Option<oval_syntax::ast::Requirement>,
        extensibility: Option<oval_syntax::ast::Extensibility>,
    ) {
        if let Some(requirement) = requirement {
            requirement.is_required();
            self.node(&requirement);
        }
        if let Some(extensibility) = extensibility {
            extensibility.is_extensible();
            self.node(&extensibility);
        }
    }

    fn ballot(&mut self, ballot: &Ballot) {
        self.node(ballot);
        self.modifiers(ballot.requirement(), ballot.extensibility());
        self.identifier(ballot.identifier());
        self.string(ballot.name());
        self.string_field(ballot.eligibility());
        self.markdown_field(ballot.note());
        for item in ballot.items() {
            match item {
                BallotItem::Section(section) => self.section(&section),
                BallotItem::Contest(contest) => self.contest(&contest),
                BallotItem::Measure(measure) => self.measure(&measure),
                BallotItem::Ballot(ballot) => self.ballot(&ballot),
            }
        }
    }

    fn section(&mut self, section: &Section) {
        self.node(section);
        self.modifiers(section.requirement(), section.extensibility());
        self.string(section.title());
        self.markdown_field(section.note());
        for item in section.items() {
            match item {
                SectionItem::Contest(contest) => self.contest(&contest),
                SectionItem::Measure(measure) => self.measure(&measure),
            }
        }
    }

    fn contest(&mut self, contest: &Contest) {
        self.node(contest);
        self.modifiers(contest.requirement(), contest.extensibility());
        self.identifier(contest.identifier());
        self.string(contest.name());
        if let Some(kind) = contest.kind() {
            self.contest_kind(&kind);
        }
        self.string_field(contest.subtitle());
        self.string_field(contest.tabulation());
        self.markdown_field(contest.note());
        for campaign in contest.campaigns() {
            self.campaign(&campaign);
        }
        for recommendation in contest.recommendations() {
            self.contest_recommend(&recommendation);
        }
    }

    fn contest_kind(&mut self, kind: &ContestKind) {
        kind.method();
        self.node(kind);
        for integer in [kind.votes(), kind.seats(), kind.min(), kind.max()]
            .into_iter()
            .flatten()
        {
            integer.value();
            self.token(&integer);
        }
        self.raw_token(kind.all());
    }

    fn campaign(&mut self, campaign: &Campaign) {
        let _ = (campaign.is_incumbent(), campaign.is_withdrawn());
        let _ = (campaign.is_party(), campaign.is_block_form());
        self.node(campaign);
        for flag in campaign.flags() {
            let _ = (flag.is_incumbent(), flag.is_withdrawn());
            self.node(&flag);
        }
        for member in campaign.members() {
            self.member(&member);
        }
        for member in campaign.party_members() {
            self.party_member(&member);
        }
        self.url_field(campaign.website());
        self.markdown_field(campaign.note());
        self.string_field(campaign.designation());
        self.party_ref(campaign.party());
    }

    fn member(&mut self, member: &Member) {
        member.is_declaration();
        self.node(member);
        self.identifier(member.identifier());
        self.string(member.name());
        self.string(member.role());
        self.string_field(member.designation());
        self.party_ref(member.party());
        self.markdown_field(member.note());
    }

    fn party_member(&mut self, member: &PartyMember) {
        member.is_declaration();
        self.node(member);
        self.identifier(member.identifier());
        self.string(member.name());
        self.string_field(member.abbreviation());
        self.url_field(member.website());
        self.markdown_field(member.note());
    }

    fn contest_recommend(&mut self, recommendation: &ContestRecommend) {
        self.node(recommendation);
        if let Some(no_stance) = recommendation.no_stance() {
            let _ = (no_stance.is_no_position(), no_stance.is_abstain());
            self.node(&no_stance);
        }
        assert_eq!(
            recommendation.stances().count(),
            recommendation.supports().count() + recommendation.opposes().count()
        );
        for stance in recommendation.stances() {
            match &stance {
                Stance::Support(support) => self.node(support),
                Stance::Oppose(oppose) => self.node(oppose),
            }
            for choice in stance.choices() {
                match choice {
                    Choice::Reference(identifier) => self.identifier(Some(identifier)),
                    Choice::WriteIn(write_in) => {
                        self.node(&write_in);
                        self.string(write_in.name());
                    }
                }
            }
        }
        self.markdown_field(recommendation.justification());
        self.markdown_field(recommendation.note());
    }

    fn measure(&mut self, measure: &Measure) {
        self.node(measure);
        self.modifiers(measure.requirement(), None);
        self.identifier(measure.identifier());
        self.string(measure.name());
        self.string_field(measure.subtitle());
        self.markdown_field(measure.description());
        self.markdown_field(measure.if_yes());
        self.markdown_field(measure.if_no());
        self.string_field(measure.placement());
        self.string_field(measure.tabulation());
        self.string_field(measure.fiscal_impact());
        self.string_list_field(measure.supporters());
        self.string_list_field(measure.opponents());
        self.markdown_field(measure.note());
        for recommendation in measure.recommendations() {
            self.measure_recommend(&recommendation);
        }
    }

    fn measure_recommend(&mut self, recommendation: &MeasureRecommend) {
        self.node(recommendation);
        if let Some(position) = recommendation.position() {
            position.value();
            self.node(&position);
        }
        self.markdown_field(recommendation.justification());
        self.markdown_field(recommendation.note());
    }
}

const NODES_WITHOUT_VIEWS: &[SyntaxKind] = &[
    SyntaxKind::Block,
    SyntaxKind::Seats,
    SyntaxKind::Minimum,
    SyntaxKind::RankedMax,
    SyntaxKind::MemberRole,
];

const VALUE_TOKENS: &[SyntaxKind] = &[
    SyntaxKind::String,
    SyntaxKind::Integer,
    SyntaxKind::Date,
    SyntaxKind::Url,
    SyntaxKind::MarkdownBlock,
];

fn is_set_aside(node: &SyntaxNode) -> bool {
    node.ancestors()
        .any(|ancestor| ancestor.kind() == SyntaxKind::ErrorNode)
}

fn unreached(text: &str) -> Vec<String> {
    let result = parse(text).expect("the text is accepted");
    let mut reached = Reached::default();
    reached.document(&result.document());
    let root = result.syntax();
    let mut missed = Vec::new();
    for node in root.descendants() {
        if !NODES_WITHOUT_VIEWS.contains(&node.kind())
            && node.kind() != SyntaxKind::ErrorNode
            && !is_set_aside(&node)
            && !reached.nodes.contains(&node)
        {
            missed.push(format!(
                "{:?}@{:?} {:?}",
                node.kind(),
                node.text_range(),
                node.text()
            ));
        }
    }
    for token in root
        .descendants_with_tokens()
        .filter_map(rowan::NodeOrToken::into_token)
    {
        let parent = token.parent().expect("every token has a parent");
        let is_value = VALUE_TOKENS.contains(&token.kind())
            || (token.kind() == SyntaxKind::Word && parent.kind() == SyntaxKind::Identifier)
            || (token.kind() == SyntaxKind::Word
                && parent.kind() == SyntaxKind::ContestKind
                && token.text() == "all");
        if is_value && !is_set_aside(&parent) && !reached.tokens.contains(&token) {
            missed.push(format!(
                "{:?}@{:?} {:?}",
                token.kind(),
                token.text_range(),
                token.text()
            ));
        }
    }
    missed
}

#[test]
fn views_reach_every_part_of_the_example_guide() {
    let missed = unreached(FRANKLIN);
    assert!(
        missed.is_empty(),
        "the views do not reach:\n{}",
        missed.join("\n")
    );
}

#[test]
fn views_reach_every_part_of_the_specification_examples() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for block in blocks(SPEC) {
        if block.info != "oval example" {
            continue;
        }
        let accepted = CONTEXTS.iter().find_map(|(_, prefix, indent)| {
            let text = in_context(&block.content, prefix, *indent);
            let result = parse(&text).expect("the example is accepted");
            let accepted = lexical_errors_are_elisions(&text, &result)
                && result
                    .syntax_errors
                    .iter()
                    .all(|error| error.spec_id() == "missing-block");
            accepted.then_some(text)
        });
        let Some(text) = accepted else {
            continue;
        };
        checked += 1;
        let missed = unreached(&text);
        if !missed.is_empty() {
            failures.push(format!(
                "SPEC.md line {}:\n{text}  not reached:\n{}",
                block.line,
                missed.join("\n")
            ));
        }
    }
    println!("walked {checked} examples through the views");
    assert!(checked > 0, "no specification examples were walked");
    assert!(
        failures.is_empty(),
        "{} of {checked} specification examples have parts the views do not reach:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}
