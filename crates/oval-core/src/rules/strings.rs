use oval_syntax::ast::{AstNode, Document};
use oval_syntax::{SyntaxElement, SyntaxKind};

use super::report;
use crate::{Diagnostic, ValidationRule};

/// Warns about strings that contain nothing, or only spaces and tabs.
pub(crate) fn check(document: &Document, out: &mut Vec<Diagnostic>) {
    let strings = document
        .syntax()
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind() == SyntaxKind::StringLiteral)
        .filter(|token| {
            !token
                .parent_ancestors()
                .any(|node| node.kind() == SyntaxKind::ErrorNode)
        });
    for token in strings {
        let text = token.text();
        let content = text
            .get(1..text.len().saturating_sub(1))
            .unwrap_or_default();
        if content
            .chars()
            .all(|character| matches!(character, ' ' | '\t'))
        {
            out.push(report(
                ValidationRule::EmptyString,
                token.text_range(),
                "this string is empty; write its text, or remove the field if it is optional"
                    .to_owned(),
            ));
        }
    }
}
