//! One document per diagnostic identifier, each producing that diagnostic at the range the
//! specification gives it.

mod support;

use std::collections::BTreeSet;

use oval_core::DiagnosticCode;
use oval_syntax::TextRange;
use support::check;

const HEAD: &str = "oval 1\nguide \"Ashford Voter Guide\"\nelection \"November 3, 2026 General Election\"\ndate 2026-11-03\n";

fn offset(source: &str, needle: &str) -> u32 {
    u32::try_from(source.find(needle).expect("needle in source")).unwrap()
}

fn span(source: &str, needle: &str) -> TextRange {
    let start = offset(source, needle);
    TextRange::new(
        start.into(),
        (start + u32::try_from(needle.len()).unwrap()).into(),
    )
}

fn span_within(source: &str, context: &str, needle: &str) -> TextRange {
    let context_start = offset(source, context);
    let inner = u32::try_from(context.find(needle).expect("needle in context")).unwrap();
    let start = context_start + inner;
    TextRange::new(
        start.into(),
        (start + u32::try_from(needle.len()).unwrap()).into(),
    )
}

fn empty_at(source: &str, needle: &str) -> TextRange {
    TextRange::empty(offset(source, needle).into())
}

fn empty_after(source: &str, needle: &str) -> TextRange {
    TextRange::empty((offset(source, needle) + u32::try_from(needle.len()).unwrap()).into())
}

fn end_of(source: &str) -> TextRange {
    TextRange::empty(u32::try_from(source.len()).unwrap().into())
}

fn document(body: &str) -> String {
    format!("{HEAD}{body}")
}

fn ballot(body: &str) -> String {
    document(&format!("ballot @city \"City of Ashford\"\n{body}"))
}

fn contest(body: &str) -> String {
    ballot(&format!("  contest @mayor \"Mayor\" single\n{body}"))
}

struct Case {
    identifier: &'static str,
    source: String,
    range: TextRange,
}

fn case(identifier: &'static str, source: String, range: impl FnOnce(&str) -> TextRange) -> Case {
    let range = range(&source);
    Case {
        identifier,
        source,
        range,
    }
}

#[allow(clippy::too_many_lines)]
fn cases() -> Vec<Case> {
    vec![
        case("illegal-tab", ballot("  measure @p \"P\"\n    subtitle\t\"S\"\n    recommend yes\n"), |s| span(s, "\t")),
        case("unexpected-character", ballot("  measure @p \"P\"\n    subtitle \"S\" $\n    recommend yes\n"), |s| span(s, "$")),
        case("unterminated-string", ballot("  measure @p \"P\"\n    subtitle \"S\n    recommend yes\n"), |s| span(s, "\"S")),
        case("invalid-escape", ballot("  measure @p \"P\"\n    subtitle \"S\\q\"\n    recommend yes\n"), |s| span(s, "\\q")),
        case("malformed-number", document("ballot @city \"City\"\n  measure @p \"P\"\n    recommend yes\n").replace("2026-11-03", "2026-11-5"), |s| span(s, "2026-11-5")),
        case("malformed-url", document("party @dem \"Democratic\"\n  website https://\ncandidate @c \"C\"\n  party @dem\nballot @city \"City\"\n  measure @p \"P\"\n    recommend yes\n"), |s| span(s, "https://")),
        case("inconsistent-dedent", ballot("  section \"Offices\"\n    measure @p \"P\"\n      recommend yes\n   measure @q \"Q\"\n     recommend no\n"), |s| span_within(s, "   measure @q", "   ")),
        case("unexpected-token", document("ballot @city \"City\"\n  measure @p \"P\"\n    recommend yes\n").replace("date 2026-11-03", "date \"November 3, 2026\""), |s| span(s, "\"November 3, 2026\"")),
        case("missing-identifier", document("party \"Democratic\"\nballot @city \"City\"\n  measure @p \"P\"\n    recommend yes\n"), |s| empty_at(s, "\"Democratic\"")),
        case("missing-name", ballot("  measure @prop-a\n    recommend yes\n"), |s| empty_after(s, "@prop-a")),
        case("missing-contest-kind", ballot("  contest @mayor \"Mayor\"\n    campaign @cfox \"Carl Fox\"\n    recommend no-position\n"), |s| empty_after(s, "\"Mayor\"")),
        case("missing-value", ballot("  measure @prop-a \"Proposition A\"\n    subtitle\n    recommend yes\n"), |s| empty_after(s, "subtitle")),
        case("missing-body", document("note:\nballot @city \"City\"\n  measure @p \"P\"\n    recommend yes\n"), |s| empty_at(s, "ballot @city")),
        case("missing-block", document("ballot @city \"City of Ashford\"\nballot @county \"Franklin County\"\n  measure @prop-a \"Proposition A\"\n    recommend yes\n"), |s| empty_at(s, "ballot @county")),
        case("unexpected-block", document("candidate @cfox \"Carl Fox\"\n  designation \"Small Business Owner\"\n    website https://fox.example\nballot @city \"City\"\n  contest @mayor \"Mayor\" single\n    campaign @cfox\n    recommend no-position\n"), |s| span(s, "website https://fox.example\n")),
        case("unexpected-entry", document("party @dem \"Democratic\"\n  designation \"D\"\ncandidate @c \"C\"\n  party @dem\nballot @city \"City\"\n  measure @p \"P\"\n    recommend yes\n"), |s| span(s, "designation \"D\"\n")),
        case("repeated-field", ballot("  measure @prop-a \"Proposition A\"\n    subtitle \"Library parcel tax renewal\"\n    subtitle \"Measure A\"\n    recommend yes\n"), |s| span(s, "subtitle \"Measure A\"\n")),
        case("repeated-flag", contest("    campaign incumbent incumbent @bsantos \"Beatriz Santos\"\n    recommend no-position\n"), |s| span_within(s, "incumbent @bsantos", "incumbent")),
        case("misplaced-entry", ballot("  measure @prop-a \"Proposition A\"\n    recommend yes\nnote:\n  Late.\n"), |s| span(s, "note:\n  Late.\n")),
        case("empty-recommendation", contest("    campaign @cfox \"Carl Fox\"\n    recommend\n      justification:\n        We could not reach either candidate.\n"), |s| span_within(s, "recommend\n      justification", "recommend")),
        case("unsupported-version", "oval 2\nguide \"G\"\n".to_owned(), |s| span(s, "oval 2")),
        case("missing-guide-field", "oval 1\nguide \"G\"\nelection \"E\"\nballot @city \"City\"\n  measure @p \"P\"\n    recommend yes\n".to_owned(), |s| empty_after(s, "oval 1")),
        case("empty-document", document(""), end_of),
        case("invalid-date", document("ballot @city \"City\"\n  measure @p \"P\"\n    recommend yes\n").replace("2026-11-03", "2026-02-30"), |s| span(s, "2026-02-30")),
        case("undeclared-reference", document("candidate @dana \"Dana\"\n  party @dem\nballot @city \"City\"\n  contest @senate \"Senator\" single\n    campaign @dana\n    recommend no-position\n"), |s| span(s, "@dem")),
        case("duplicate-declaration", document("party @dem \"Democratic\"\nparty @dem \"Democrats\"\ncandidate @c \"C\"\n  party @dem\nballot @city \"City\"\n  contest @m \"M\" single\n    campaign @c\n    recommend no-position\n"), |s| span_within(s, "party @dem \"Democrats\"", "@dem")),
        case("mismatched-reference", document("candidate @dana \"Dana\"\n  party @sofia\ncandidate @sofia \"Sofia\"\nballot @city \"City\"\n  contest @m \"M\" single\n    campaign @dana\n    campaign @sofia\n    recommend no-position\n"), |s| span_within(s, "party @sofia", "@sofia")),
        case("block-on-reference", document("candidate @dana \"Dana\"\nballot @city \"City\"\n  contest @m \"M\" single\n    campaign\n      @dana as \"President\"\n        designation \"Senator\"\n    recommend no-position\n"), |s| span(s, "designation \"Senator\"\n")),
        case("misplaced-candidate-field", document("candidate @dana \"Dana\"\nballot @city \"City\"\n  contest @m \"M\" single\n    campaign @dana\n      designation \"Senator\"\n    recommend no-position\n"), |s| span(s, "designation \"Senator\"\n")),
        case("empty-ballot", document("ballot @city \"City of Ashford\"\n  eligibility \"Residents\"\nballot @county \"County\"\n  measure @p \"P\"\n    recommend yes\n"), |s| span(s, "ballot @city \"City of Ashford\"")),
        case("empty-section", ballot("  section \"Measures\"\n    note:\n      None this year.\n  measure @p \"P\"\n    recommend yes\n"), |s| span(s, "section \"Measures\"")),
        case("contest-without-campaigns", ballot("  contest @mayor \"Mayor\" single\n    recommend no-position\n"), |s| span(s, "contest @mayor \"Mayor\" single")),
        case("campaign-without-members", contest("    campaign\n      website https://fox.example\n    recommend no-position\n"), |s| span_within(s, "campaign\n      website", "campaign")),
        case("duplicate-member", contest("    campaign\n      @hana \"Hana\" as \"President\"\n      @hana as \"Vice President\"\n    recommend no-position\n"), |s| span(s, "@hana as \"Vice President\"\n")),
        case("candidate-in-two-campaigns", contest("    campaign @bsantos \"Beatriz Santos\"\n    campaign @bsantos\n    recommend no-position\n"), |s| span_within(s, "campaign @bsantos\n", "@bsantos")),
        case("invalid-multiple-count", ballot("  contest @council \"City Council\" multiple 1\n    campaign @a \"A\"\n    recommend no-position\n"), |s| span_within(s, "multiple 1", "1")),
        case("invalid-ranked-max", ballot("  contest @board \"School Board\" ranked max 0\n    campaign @a \"A\"\n    recommend no-position\n"), |s| span_within(s, "max 0", "0")),
        case("retention-campaign-count", ballot("  contest @judge \"Retain Judge Ueda?\" retention\n    campaign incumbent @ueda \"Mika Ueda\"\n    campaign @cfox \"Carl Fox\"\n    recommend\n      support @ueda\n"), |s| span(s, "campaign @cfox \"Carl Fox\"\n")),
        case("missing-recommendation", ballot("  measure @prop-a \"Proposition A\"\n    subtitle \"Library parcel tax renewal\"\n"), |s| span(s, "measure @prop-a \"Proposition A\"")),
        case("repeated-recommendation", ballot("  measure @prop-a \"Proposition A\"\n    recommend yes\n    recommend no\n"), |s| span(s, "recommend no\n")),
        case("campaign-not-in-contest", document("candidate @cfox \"Carl Fox\"\nballot @city \"City\"\n  contest @mayor \"Mayor\" single\n    campaign @bsantos \"Beatriz Santos\"\n    recommend\n      support @cfox\n  contest @clerk \"Clerk\" single\n    campaign @cfox\n    recommend no-position\n"), |s| span_within(s, "support @cfox", "@cfox")),
        case("repeated-oppose", contest("    campaign @bsantos \"Beatriz Santos\"\n    campaign @cfox \"Carl Fox\"\n    campaign @dwright \"Dee Wright\"\n    recommend\n      support @bsantos\n      oppose @cfox\n      oppose @dwright\n"), |s| span(s, "oppose @dwright\n")),
        case("repeated-stance", contest("    campaign @bsantos \"Beatriz Santos\"\n    campaign @cfox \"Carl Fox\"\n    recommend\n      support @bsantos\n      oppose @bsantos\n"), |s| span_within(s, "oppose @bsantos", "@bsantos")),
        case("tiers-in-single", contest("    campaign @bsantos \"Beatriz Santos\"\n    campaign @cfox \"Carl Fox\"\n    recommend\n      support @bsantos\n      support @cfox\n"), |s| span(s, "support @cfox\n")),
        case("support-over-max", ballot("  contest @board \"School Board\" ranked max 2\n    campaign @aokafor \"Ada Okafor\"\n    campaign @bsantos \"Beatriz Santos\"\n    campaign @cfox \"Carl Fox\"\n    recommend\n      support @aokafor\n      support @bsantos\n      support @cfox\n"), |s| span_within(s, "support @cfox", "@cfox")),
        case("retention-stance", ballot("  contest @judge \"Retain Judge Ueda?\" retention\n    campaign incumbent @ueda \"Mika Ueda\"\n    recommend\n      support @ueda\n      oppose @ueda\n"), |s| span(s, "oppose @ueda\n")),
        case("markdown-profile", ballot("  measure @prop-a \"Proposition A\"\n    note:\n      Run `oval check` first.\n    recommend yes\n"), |s| span(s, "`oval check`")),
        case("unused-declaration", document("party @dem \"Democratic\"\nparty @gop \"Republican\"\ncandidate @dana \"Dana\"\n  party @dem\nballot @city \"City\"\n  contest @m \"M\" single\n    campaign @dana\n    recommend no-position\n"), |s| span(s, "party @gop \"Republican\"")),
        case("duplicate-party-name", document("party @dem \"Democratic\"\nparty @dems \"Democratic\"\ncandidate @a \"A\"\n  party @dem\ncandidate @b \"B\"\n  party @dems\nballot @city \"City\"\n  contest @m \"M\" single\n    campaign @a\n    campaign @b\n    recommend no-position\n"), |s| span_within(s, "party @dems \"Democratic\"", "\"Democratic\"")),
        case("duplicate-ballot-title", document("ballot @city \"City of Ashford\"\n  measure @p \"P\"\n    recommend yes\nballot @city-2 \"City of Ashford\"\n  measure @q \"Q\"\n    recommend no\n"), |s| span_within(s, "ballot @city-2 \"City of Ashford\"", "\"City of Ashford\"")),
        case("support-over-count", ballot("  contest @council \"City Council\" multiple 2\n    campaign @aokafor \"Ada Okafor\"\n    campaign @bsantos \"Beatriz Santos\"\n    campaign @cfox \"Carl Fox\"\n    recommend\n      support @aokafor @bsantos\n      support @cfox\n"), |s| span_within(s, "support @cfox", "@cfox")),
        case("non-head-reference", contest("    campaign\n      @hana \"Hana\" as \"President\"\n      @hiro \"Hiro\" as \"Vice President\"\n    recommend\n      support @hiro\n"), |s| span_within(s, "support @hiro", "@hiro")),
        case("insecure-url", document("party @dem \"Democratic\"\n  website http://democrats.example\ncandidate @c \"C\"\n  party @dem\nballot @city \"City\"\n  contest @m \"M\" single\n    campaign @c\n    recommend no-position\n"), |s| span(s, "http://democrats.example")),
        case("empty-string", ballot("  measure @prop-a \"Proposition A\"\n    subtitle \"\"\n    recommend yes\n"), |s| span(s, "\"\"")),
        case("unpaired-outcome", ballot("  measure @prop-a \"Proposition A\"\n    if_yes:\n      The tax continues.\n    recommend yes\n"), |s| span(s, "if_yes:\n      The tax continues.\n")),
    ]
}

#[test]
fn every_identifier_has_a_case_and_every_case_reports_its_identifier_once() {
    let cases = cases();
    let covered: BTreeSet<&str> = cases.iter().map(|case| case.identifier).collect();
    let all: BTreeSet<&str> = DiagnosticCode::all()
        .into_iter()
        .map(DiagnosticCode::identifier)
        .collect();
    assert_eq!(covered, all);
    for case in cases {
        let report = check(&case.source);
        let found: Vec<TextRange> = report
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code.identifier() == case.identifier)
            .map(|diagnostic| diagnostic.range)
            .collect();
        assert_eq!(
            found,
            [case.range],
            "{}: {:?}\n{}",
            case.identifier,
            report
                .diagnostics()
                .iter()
                .map(|d| (d.code.identifier(), d.range, d.message.as_str()))
                .collect::<Vec<_>>(),
            case.source
        );
    }
}

#[test]
fn body_positions_map_back_to_the_source_whatever_the_terminator() {
    for terminator in ["\n", "\r\n", "\r"] {
        let source = ballot(&format!(
            "  measure @p \"P\"\n    note:{terminator}      First.{terminator}{terminator}      Then `code`.{terminator}    recommend yes\n"
        ));
        let report = check(&source);
        let found: Vec<TextRange> = report
            .diagnostics()
            .iter()
            .filter(|d| d.code.identifier() == "markdown-profile")
            .map(|d| d.range)
            .collect();
        assert_eq!(found, [span(&source, "`code`")], "{terminator:?}");
    }
}

#[test]
fn the_markdown_profile_names_each_construct_it_rejects() {
    let body = "    See <https://a.example> and [the text](https://b.example/t).\n\n    # A heading\n\n    | a | b |\n    |---|---|\n    | 1 | 2 |\n\n    > quoted\n\n    1. one\n    2. two\n";
    let clean = ballot(&format!(
        "  measure @p \"P\"\n    note:\n{body}    recommend yes\n"
    ));
    let report = check(&clean);
    assert!(
        report
            .diagnostics()
            .iter()
            .all(|d| d.code.identifier() != "markdown-profile"),
        "{:?}",
        report.diagnostics()
    );

    let rejected = [
        (
            "```\n      x\n      ```",
            "code is not part of the Markdown profile",
        ),
        ("a<br>b", "raw HTML is not part of the Markdown profile"),
        (
            "![alt](https://a.example/i.png)",
            "images are not part of the Markdown profile",
        ),
        (
            "---",
            "thematic breaks are not part of the Markdown profile",
        ),
        (
            "[rel](/relative)",
            "a link destination must be an absolute `http` or `https` URL",
        ),
        (
            "[ref][r]\n\n      [r]: ftp://a.example",
            "a link destination must be an absolute `http` or `https` URL",
        ),
    ];
    for (construct, message) in rejected {
        let source = ballot(&format!(
            "  measure @p \"P\"\n    note:\n      {construct}\n    recommend yes\n"
        ));
        let report = check(&source);
        let messages: Vec<&str> = report
            .diagnostics()
            .iter()
            .filter(|d| d.code.identifier() == "markdown-profile")
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(messages, [message], "{construct}");
    }

    let source = ballot(
        "  measure @p \"P\"\n    note:\n      [site](http://a.example)\n    recommend yes\n",
    );
    let report = check(&source);
    let insecure: Vec<TextRange> = report
        .diagnostics()
        .iter()
        .filter(|d| d.code.identifier() == "insecure-url")
        .map(|d| d.range)
        .collect();
    assert_eq!(insecure, [span(&source, "http://a.example")]);
}
