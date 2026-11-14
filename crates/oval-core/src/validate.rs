use oval_syntax::ast::Document;
use oval_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TokenKind};

use crate::{Diagnostic, DiagnosticCode, ValidationRule};

mod document;
mod literals;
mod version;

pub(crate) fn validate(document: &Document) -> Result<Vec<Diagnostic>, Diagnostic> {
    let marker = version::supported_marker(document)?;
    let mut diagnostics = Vec::new();
    document::guide_fields(document, &marker, &mut diagnostics);
    document::root_ballots(document, &mut diagnostics);
    literals::literals(document, &mut diagnostics);
    Ok(diagnostics)
}

fn kept_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + use<> {
    node.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| {
            !token
                .parent_ancestors()
                .any(|ancestor| ancestor.kind() == SyntaxKind::ErrorNode)
        })
}

fn content_range(node: &SyntaxNode) -> Option<TextRange> {
    let mut ranges = node
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind().token_kind().is_some_and(TokenKind::is_lexical))
        .map(|token| token.text_range());
    let first = ranges.next()?;
    Some(ranges.fold(first, TextRange::cover))
}

fn diagnostic(rule: ValidationRule, range: TextRange) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::Validation(rule), range)
}

#[cfg(test)]
pub(super) const VALID: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n";

#[cfg(test)]
fn reported(text: &str) -> Vec<(&'static str, u32, u32)> {
    crate::check(text)
        .unwrap()
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code.spec_id(),
                diagnostic.range.start().into(),
                diagnostic.range.end().into(),
            )
        })
        .collect()
}

#[cfg(test)]
fn reported_as(text: &str, id: &str) -> Vec<(u32, u32)> {
    reported(text)
        .into_iter()
        .filter(|(reported_id, _, _)| *reported_id == id)
        .map(|(_, start, end)| (start, end))
        .collect()
}

#[cfg(test)]
mod tests {
    use oval_syntax::TextRange;

    use super::{VALID, content_range, kept_tokens, reported};

    fn range(text: &str) -> Option<TextRange> {
        content_range(&oval_syntax::parse(text).unwrap().syntax())
    }

    #[test]
    fn a_valid_document_breaks_no_rule() {
        assert_eq!(reported(VALID), []);
    }

    #[test]
    fn content_runs_from_the_first_to_the_last_token_the_parser_does_not_skip() {
        assert_eq!(
            range("\n  ~ oval 1 ~  # c\n"),
            Some(TextRange::new(5.into(), 11.into()))
        );
    }

    #[test]
    fn tokens_in_discarded_or_set_aside_text_are_not_kept() {
        let text = "guide \"G\" 5\nguide \"H\"\n";
        let kept: Vec<String> = kept_tokens(&oval_syntax::parse(text).unwrap().syntax())
            .map(|token| token.text().to_owned())
            .collect();
        assert_eq!(kept, ["guide", " ", "\"G\"", " ", "\n"]);
    }

    #[test]
    fn a_document_of_only_skipped_tokens_has_no_content() {
        assert_eq!(range(""), None);
        assert_eq!(range("# c\n\n~\n"), None);
    }
}
