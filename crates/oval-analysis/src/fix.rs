use oval_core::{DiagnosticCode, ValidationRule};
use oval_syntax::ast::{AstNode, Document};
use oval_syntax::{
    SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize, TokenKind,
};

use crate::Analysis;
use crate::symbols::nameable;

/// A change to a document's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// The text to replace, which is empty for an insertion.
    pub range: TextRange,
    /// The text to put in its place.
    pub text: String,
    /// The part of `text` the author should fill in, as a range within `text`, or `None` when the
    /// text is complete.
    pub placeholder: Option<TextRange>,
}

/// A change that repairs one diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    /// The position of the diagnostic in the report's
    /// [`diagnostics`](oval_core::Report::diagnostics).
    pub diagnostic: usize,
    /// What the fix does, in a few words, such as ``Change to `@dem` ``.
    pub title: String,
    /// The change to make.
    pub edit: Edit,
}

impl Analysis {
    /// Returns a fix for each diagnostic that has a mechanical repair, in the order of the
    /// diagnostics.
    ///
    /// Five diagnostics have one.
    ///
    /// - An `insecure-url` changes `http` to `https`.
    /// - An `undeclared-reference` close in spelling to a name the reference may use is changed to
    ///   that name.
    /// - A `missing-guide-field` is added on its own line after the version marker and any guide
    ///   field that comes before it, with its value left for the author to fill in.
    /// - A `missing-recommendation` adds `recommend no-position` as the last line of the contest
    ///   or measure.
    /// - An `unused-declaration` removes the declaration, unless something outside it names a
    ///   party it declares.
    ///
    /// An added line ends with the document's own line terminator.
    pub fn fixes(&self) -> Vec<Fix> {
        let source = self.report().parse().syntax().to_string();
        let document = self.report().parse().document();
        let mut absent = absent_guide_fields(&document).into_iter();
        let mut fixes = Vec::new();
        for (position, diagnostic) in self.report().diagnostics().iter().enumerate() {
            let DiagnosticCode::Validation(rule) = diagnostic.code else {
                continue;
            };
            let range = diagnostic.range;
            let fix = match rule {
                ValidationRule::InsecureUrl => Some(insecure_url(range)),
                ValidationRule::UndeclaredReference => self.undeclared_reference(range),
                ValidationRule::MissingGuideField => absent
                    .next()
                    .and_then(|field| guide_field(&source, &document, field)),
                ValidationRule::MissingRecommendation => {
                    self.missing_recommendation(&source, range)
                }
                ValidationRule::UnusedDeclaration => self.unused_declaration(range),
                _ => None,
            };
            if let Some((title, edit)) = fix {
                fixes.push(Fix {
                    diagnostic: position,
                    title,
                    edit,
                });
            }
        }
        fixes
    }

    fn undeclared_reference(&self, range: TextRange) -> Option<(String, Edit)> {
        let symbols = self.symbols();
        let reference = symbols
            .references()
            .iter()
            .find(|reference| reference.identifier.syntax().text_range() == range)?;
        let written = reference.identifier.name()?;
        let nearest = nameable(symbols, reference)
            .into_iter()
            .filter_map(|declaration| Some(declaration.identifier.name()?.text().to_owned()))
            .map(|name| (distance(written.text(), &name), name))
            .filter(|(distance, name)| *distance <= (name.len() / 3).max(1))
            .min_by_key(|(distance, name)| (*distance, name.len()))
            .map(|(_, name)| name)?;
        Some((
            format!("Change to `@{nearest}`"),
            edit(written.text_range(), nearest),
        ))
    }

    fn missing_recommendation(&self, source: &str, range: TextRange) -> Option<(String, Edit)> {
        let construct = self.node_at(range, &[SyntaxKind::Contest, SyntaxKind::Measure])?;
        let indentation = construct
            .children()
            .find(|child| child.kind() == SyntaxKind::Block)
            .map_or_else(
                || column(source, construct.text_range().start()) + 2,
                |block| column(source, block.text_range().start()),
            );
        let line = format!("{}recommend no-position", " ".repeat(indentation));
        let (insertion, _) = add_line(source, &last_lexical_token(&construct)?, &line);
        Some(("Add `recommend no-position`".to_owned(), insertion))
    }

    fn unused_declaration(&self, range: TextRange) -> Option<(String, Edit)> {
        let declaration = self.node_at(range, &[SyntaxKind::Party, SyntaxKind::Candidate])?;
        let extent = declaration.text_range();
        let symbols = self.symbols();
        let named_from_outside = symbols
            .references()
            .iter()
            .filter(|reference| !extent.contains_range(reference.identifier.syntax().text_range()))
            .filter_map(|reference| symbols.resolve(&reference.identifier))
            .any(|position| {
                extent.contains_range(
                    symbols.declarations()[position]
                        .identifier
                        .syntax()
                        .text_range(),
                )
            });
        if named_from_outside {
            return None;
        }
        let (end, _) = line_end(&last_lexical_token(&declaration)?);
        let identifier = declaration
            .children()
            .find(|child| child.kind() == SyntaxKind::Identifier)?;
        Some((
            format!("Remove `{identifier}`"),
            edit(TextRange::new(extent.start(), end), String::new()),
        ))
    }

    fn node_at(&self, range: TextRange, kinds: &[SyntaxKind]) -> Option<SyntaxNode> {
        let element = self.report().parse().syntax().covering_element(range);
        let node = match element {
            SyntaxElement::Node(node) => node,
            SyntaxElement::Token(token) => token.parent()?,
        };
        node.ancestors().find(|node| kinds.contains(&node.kind()))
    }
}

fn insecure_url(range: TextRange) -> (String, Edit) {
    let scheme = TextRange::at(range.start(), TextSize::of("http"));
    ("Use `https`".to_owned(), edit(scheme, "https".to_owned()))
}

#[derive(Clone, Copy)]
enum GuideField {
    Guide,
    Election,
    Date,
}

fn absent_guide_fields(document: &Document) -> Vec<GuideField> {
    [
        (GuideField::Guide, document.guide().is_none()),
        (GuideField::Election, document.election().is_none()),
        (GuideField::Date, document.date().is_none()),
    ]
    .into_iter()
    .filter_map(|(field, absent)| absent.then_some(field))
    .collect()
}

fn guide_field(source: &str, document: &Document, field: GuideField) -> Option<(String, Edit)> {
    let marker = document
        .version_marker()
        .map(|marker| marker.syntax().clone());
    let guide = document.guide().map(|guide| guide.syntax().clone());
    let election = document
        .election()
        .map(|election| election.syntax().clone());
    let (keyword, value, before) = match field {
        GuideField::Guide => ("guide", "\"\"", vec![marker]),
        GuideField::Election => ("election", "\"\"", vec![marker, guide]),
        GuideField::Date => ("date", "YYYY-MM-DD", vec![marker, guide, election]),
    };
    let anchor = before
        .into_iter()
        .flatten()
        .max_by_key(|node| node.text_range().end())?;
    let line = format!("{keyword} {value}");
    let (mut insertion, line_start) = add_line(source, &last_lexical_token(&anchor)?, &line);
    let value_start = line_start + TextSize::of(keyword) + TextSize::of(" ");
    insertion.placeholder = Some(if value == "\"\"" {
        TextRange::empty(value_start + TextSize::of("\""))
    } else {
        TextRange::at(value_start, TextSize::of(value))
    });
    Some((format!("Add `{keyword}`"), insertion))
}

fn edit(range: TextRange, text: String) -> Edit {
    Edit {
        range,
        text,
        placeholder: None,
    }
}

fn add_line(source: &str, token: &SyntaxToken, line: &str) -> (Edit, TextSize) {
    let terminator = terminator(source);
    let (end, terminated) = line_end(token);
    let (text, line_start) = if terminated {
        (format!("{line}{terminator}"), TextSize::from(0))
    } else {
        (format!("{terminator}{line}"), TextSize::of(terminator))
    };
    (edit(TextRange::empty(end), text), line_start)
}

fn line_end(token: &SyntaxToken) -> (TextSize, bool) {
    let end = std::iter::successors(Some(token.clone()), SyntaxToken::next_token)
        .find(|token| {
            matches!(
                token.kind(),
                SyntaxKind::Newline | SyntaxKind::MarkdownBlock
            )
        })
        .unwrap_or_else(|| token.clone());
    let terminated = end.text().ends_with(['\n', '\r']);
    (end.text_range().end(), terminated)
}

fn last_lexical_token(node: &SyntaxNode) -> Option<SyntaxToken> {
    node.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind().token_kind().is_some_and(TokenKind::is_lexical))
        .last()
}

fn column(source: &str, offset: TextSize) -> usize {
    let offset = usize::from(offset);
    let line_start = source[..offset].rfind(['\n', '\r']).map_or(0, |at| at + 1);
    offset - line_start
}

fn terminator(source: &str) -> &'static str {
    match source.find(['\n', '\r']) {
        Some(at) if source[at..].starts_with("\r\n") => "\r\n",
        Some(at) if source[at..].starts_with('\r') => "\r",
        _ => "\n",
    }
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut rows = vec![vec![0; b.len() + 1]; a.len() + 1];
    rows[0] = (0..=b.len()).collect();
    for (i, row) in rows.iter_mut().enumerate() {
        row[0] = i;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let substitution = rows[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1]);
            let mut best = substitution.min(rows[i - 1][j] + 1).min(rows[i][j - 1] + 1);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = best;
        }
    }
    rows[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use oval_syntax::{TextRange, TextSize};

    use super::Fix;
    use crate::Analysis;

    const GUIDE: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\n";

    fn fixes(source: &str, code: &str) -> Vec<Fix> {
        let analysis = Analysis::new(source).unwrap();
        analysis
            .fixes()
            .into_iter()
            .filter(|fix| {
                analysis.report().diagnostics()[fix.diagnostic]
                    .code
                    .spec_id()
                    == code
            })
            .collect()
    }

    fn fix(source: &str, code: &str) -> Fix {
        let mut fixes = fixes(source, code);
        assert_eq!(fixes.len(), 1, "{fixes:?}");
        fixes.remove(0)
    }

    fn apply(source: &str, fix: &Fix) -> String {
        let mut fixed = source.to_owned();
        fixed.replace_range(Range::<usize>::from(fix.edit.range), &fix.edit.text);
        fixed
    }

    fn contest(body: &str) -> String {
        format!(
            "{GUIDE}party @dem \"Democratic\"\ncandidate @nadia \"Nadia\"\ncandidate @xavier \"Xavier\"\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @nadia\n    campaign party @dem\n    campaign @xavier\n{body}"
        )
    }

    #[test]
    fn an_insecure_url_uses_https() {
        let source = format!("{GUIDE}party @p \"P\"\n  website http://example.com\n");
        let fix = fix(&source, "insecure-url");
        assert_eq!(fix.title, "Use `https`");
        assert!(apply(&source, &fix).contains("  website https://example.com\n"));
    }

    #[test]
    fn a_misspelled_reference_is_changed_to_the_closest_name_it_may_use() {
        let source = contest("    recommend\n      support @nadai\n      oppose @dme\n");
        let fixes = fixes(&source, "undeclared-reference");
        let titles: Vec<&str> = fixes.iter().map(|fix| fix.title.as_str()).collect();
        assert_eq!(titles, ["Change to `@nadia`", "Change to `@dem`"]);
        assert!(apply(&source, &fixes[0]).contains("support @nadia\n"));
        let start = TextSize::try_from(source.find("nadai").unwrap()).unwrap();
        assert_eq!(
            fixes[0].edit.range,
            TextRange::at(start, TextSize::of("nadai"))
        );
    }

    #[test]
    fn a_reference_far_from_every_name_it_may_use_has_no_fix() {
        let far = contest("    recommend\n      support @zed\n");
        assert_eq!(fixes(&far, "undeclared-reference"), []);
        let source = format!(
            "{GUIDE}party @dem \"D\"\ncandidate @n \"N\"\n  party @nn\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @n\n    recommend no-position\n"
        );
        assert_eq!(fixes(&source, "undeclared-reference"), []);
    }

    #[test]
    fn a_misspelled_choice_is_only_changed_to_a_member_of_the_contest() {
        let source = format!(
            "{GUIDE}candidate @jordan \"Jordan\"\ncandidate @jordyn \"Jordyn\"\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @jordyn\n    recommend\n      support @jordam\n"
        );
        assert_eq!(
            fix(&source, "undeclared-reference").title,
            "Change to `@jordyn`"
        );
    }

    #[test]
    fn a_missing_guide_field_is_added_after_the_fields_before_it() {
        let source = "oval 1  # version\nelection \"E\"\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n";
        let fixes = fixes(source, "missing-guide-field");
        let titles: Vec<&str> = fixes.iter().map(|fix| fix.title.as_str()).collect();
        assert_eq!(titles, ["Add `guide`", "Add `date`"]);
        assert!(
            apply(source, &fixes[0]).starts_with("oval 1  # version\nguide \"\"\nelection \"E\"\n")
        );
        assert!(
            apply(source, &fixes[1])
                .starts_with("oval 1  # version\nelection \"E\"\ndate YYYY-MM-DD\n")
        );
        assert_eq!(
            fixes[0].edit.placeholder,
            Some(TextRange::empty(TextSize::of("guide \"")))
        );
        assert_eq!(
            fixes[1].edit.placeholder,
            Some(TextRange::at(
                TextSize::of("date "),
                TextSize::of("YYYY-MM-DD")
            ))
        );
    }

    #[test]
    fn a_guide_field_added_at_the_end_of_the_document_begins_with_a_terminator() {
        let source = "oval 1\r\nguide \"G\"\r\nelection \"E\"";
        let fix = fix(source, "missing-guide-field");
        assert_eq!(fix.edit.text, "\r\ndate YYYY-MM-DD");
        assert_eq!(
            fix.edit.placeholder,
            Some(TextRange::at(
                TextSize::of("\r\ndate "),
                TextSize::of("YYYY-MM-DD")
            ))
        );
        assert!(apply(source, &fix).ends_with("election \"E\"\r\ndate YYYY-MM-DD"));
    }

    #[test]
    fn a_missing_recommendation_is_added_as_the_last_line_of_the_block() {
        let source = format!(
            "{GUIDE}ballot @b \"B\"\n  contest @k \"K\" single\n      campaign @a \"A\"\n        note:\n          Body.\n\n      # about the measure\n  measure @m \"M\"\n    subtitle \"S\""
        );
        let fixes = fixes(&source, "missing-recommendation");
        assert_eq!(fixes.len(), 2);
        assert_eq!(fixes[0].title, "Add `recommend no-position`");
        assert!(apply(&source, &fixes[0]).contains(
            "          Body.\n      recommend no-position\n\n      # about the measure\n"
        ));
        assert!(
            apply(&source, &fixes[1]).ends_with("    subtitle \"S\"\n    recommend no-position")
        );
    }

    #[test]
    fn a_missing_recommendation_without_a_block_is_indented_below_its_header() {
        let source = format!("{GUIDE}ballot @b \"B\"\n  measure @m \"M\"\n");
        let fix = fix(&source, "missing-recommendation");
        assert!(apply(&source, &fix).ends_with("  measure @m \"M\"\n    recommend no-position\n"));
    }

    #[test]
    fn an_unused_declaration_is_removed_with_its_block() {
        let source = format!(
            "{GUIDE}party @gop \"Republican\"  # unused\n  note:\n    Body.\nparty @dem \"Democratic\"\n"
        );
        let fixes = fixes(&source, "unused-declaration");
        let titles: Vec<&str> = fixes.iter().map(|fix| fix.title.as_str()).collect();
        assert_eq!(titles, ["Remove `@gop`", "Remove `@dem`"]);
        assert!(
            apply(&source, &fixes[0]).ends_with("date 2026-11-03\nparty @dem \"Democratic\"\n")
        );
        assert!(apply(&source, &fixes[1]).ends_with("    Body.\n"));
    }

    #[test]
    fn a_declaration_that_declares_a_party_named_elsewhere_is_not_removed() {
        let source = format!(
            "{GUIDE}candidate @c \"C\"\n  party @grn \"Green\"\ncandidate @d \"D\"\n  party @grn\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @d\n    recommend no-position\n"
        );
        assert_eq!(fixes(&source, "unused-declaration"), []);
    }

    #[test]
    fn a_fix_names_its_diagnostic_and_others_have_none() {
        let source = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-02-30\n";
        let analysis = Analysis::new(source).unwrap();
        let fixes = analysis.fixes();
        let codes: Vec<&str> = analysis
            .report()
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.spec_id())
            .collect();
        assert_eq!(codes, ["invalid-date", "empty-document"]);
        assert_eq!(fixes, []);
        let source = format!("{GUIDE}party @p \"P\"\n  website http://example.com\n");
        let analysis = Analysis::new(&source).unwrap();
        let fixes = analysis.fixes();
        let fix = fixes.iter().find(|fix| fix.title == "Use `https`").unwrap();
        let diagnostic = &analysis.report().diagnostics()[fix.diagnostic];
        assert_eq!(diagnostic.code.spec_id(), "insecure-url");
        assert_eq!(
            &source[Range::<usize>::from(diagnostic.range)],
            "http://example.com"
        );
    }
}
