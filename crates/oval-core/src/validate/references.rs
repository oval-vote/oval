use oval_syntax::SyntaxKind;
use oval_syntax::ast::{AstNode, AstToken};

use super::index::{Entity, Index};
use super::{content_range, diagnostic, header_range};
use crate::{Diagnostic, ValidationRule};

pub(super) fn duplicate_declarations(index: &Index, diagnostics: &mut Vec<Diagnostic>) {
    for (position, declaration) in index.declarations.iter().enumerate() {
        if index.resolve(&declaration.identifier) != Some(position) {
            diagnostics.push(diagnostic(
                ValidationRule::DuplicateDeclaration,
                declaration.identifier.syntax().text_range(),
            ));
        }
    }
}

pub(super) fn references(index: &Index, diagnostics: &mut Vec<Diagnostic>) {
    for reference in &index.references {
        let rule = match index.resolve(&reference.identifier) {
            None => ValidationRule::UndeclaredReference,
            Some(declaration)
                if !reference
                    .position
                    .accepts(index.declarations[declaration].entity) =>
            {
                ValidationRule::MismatchedReference
            }
            Some(_) => continue,
        };
        diagnostics.push(diagnostic(rule, reference.identifier.syntax().text_range()));
    }
}

pub(super) fn blocks_on_references(index: &Index, diagnostics: &mut Vec<Diagnostic>) {
    for reference in &index.references {
        let Some(block) = reference.identifier.syntax().parent().and_then(|node| {
            node.children()
                .find(|child| child.kind() == SyntaxKind::Block)
        }) else {
            continue;
        };
        if block
            .children()
            .all(|entry| entry.kind() == SyntaxKind::ErrorNode)
        {
            continue;
        }
        if let Some(range) = content_range(&block) {
            diagnostics.push(diagnostic(ValidationRule::BlockOnReference, range));
        }
    }
}

pub(super) fn duplicate_party_names(index: &Index, diagnostics: &mut Vec<Diagnostic>) {
    let mut earlier: Vec<(String, String)> = Vec::new();
    let parties = index
        .declarations
        .iter()
        .filter(|declaration| declaration.entity == Entity::Party);
    for party in parties {
        let (Some(name), Some(identifier)) = (party.name(), party.identifier.name()) else {
            continue;
        };
        let (name_text, identifier_text) =
            (name.value().into_owned(), identifier.text().to_owned());
        if earlier.iter().any(|(other_name, other_identifier)| {
            *other_name == name_text && *other_identifier != identifier_text
        }) {
            diagnostics.push(diagnostic(
                ValidationRule::DuplicatePartyName,
                name.syntax().text_range(),
            ));
        }
        earlier.push((name_text, identifier_text));
    }
}

pub(super) fn unused_declarations(index: &Index, diagnostics: &mut Vec<Diagnostic>) {
    let mut used = vec![false; index.declarations.len()];
    for reference in &index.references {
        if reference.position.is_use()
            && let Some(declaration) = index.resolve(&reference.identifier)
        {
            used[declaration] = true;
        }
    }
    for (declaration, used) in index.declarations.iter().zip(used) {
        let Some(node) = declaration.identifier.syntax().parent() else {
            continue;
        };
        if !used && matches!(node.kind(), SyntaxKind::Party | SyntaxKind::Candidate) {
            diagnostics.push(diagnostic(
                ValidationRule::UnusedDeclaration,
                header_range(&node).expect("a declaration begins with its keyword"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::validate::{reported, reported_as};

    fn guide(body: &str) -> String {
        format!("oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\n{body}")
    }

    #[test]
    fn each_declaration_after_the_first_is_a_duplicate() {
        let text = guide(
            "party @dem \"Democratic\"\nparty @dem \"Democrats\"\ncandidate @dem \"Dana\"\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n",
        );
        assert_eq!(
            reported_as(&text, "duplicate-declaration"),
            [(76, 80), (103, 107)]
        );
    }

    #[test]
    fn every_kind_of_declaration_shares_one_namespace() {
        let text = guide(
            "ballot @x \"B\"\n  measure @x \"M\"\n    recommend yes\n  contest @c \"C\" single\n    campaign @x \"X\"\n    recommend no-position\n",
        );
        assert_eq!(
            reported_as(&text, "duplicate-declaration"),
            [(70, 72), (132, 134)]
        );
    }

    #[test]
    fn a_reference_to_no_declaration_is_undeclared() {
        let text = guide(
            "candidate @c \"C\"\n  party @dem\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @c\n    campaign party @gop\n    recommend\n      support @nobody\n",
        );
        assert_eq!(
            reported_as(&text, "undeclared-reference"),
            [(71, 75), (149, 153), (182, 189)]
        );
    }

    #[test]
    fn a_reference_to_a_kind_its_position_does_not_accept_is_mismatched() {
        let text = guide(
            "party @dem \"D\"\ncandidate @c \"C\"\n  party @c\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @dem\n    campaign party @c\n    recommend\n      support @b\n",
        );
        assert_eq!(
            reported_as(&text, "mismatched-reference"),
            [(86, 88), (140, 144), (164, 166), (195, 197)]
        );
    }

    #[test]
    fn a_reference_resolves_to_an_inline_declaration_anywhere_in_the_document() {
        let text = guide(
            "ballot @b \"B\"\n  contest @k \"K\" single\n    campaign @kobi\n    campaign @kobi \"Kwame Obi\"\n      party @green \"Green\"\n    campaign party @green\n    recommend no-position\n",
        );
        assert_eq!(reported(&text), []);
    }

    #[test]
    fn a_block_of_fields_below_a_reference_is_invalid() {
        let text = guide(
            "party @dem \"Dem\"\ncandidate @n \"N\"\n  party @dem\n    abbreviation \"D\"\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign\n      @n as \"P\"\n        designation \"S\"\n      @x \"X\"\n        designation \"S\"\n    campaign party\n      @dem\n        abbreviation \"D\"\n    recommend no-position\n",
        );
        assert_eq!(
            reported_as(&text, "block-on-reference"),
            [(97, 113), (189, 204), (280, 296)]
        );
    }

    #[test]
    fn a_block_of_only_discarded_text_below_a_reference_is_not_reported() {
        let text = guide(
            "party @dem \"Dem\"\ncandidate @n \"N\"\n  party @dem\n    bogus 1\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @n\n    recommend no-position\n",
        );
        assert_eq!(reported(&text), [("unexpected-entry", 97, 104)]);
    }

    #[test]
    fn parties_declared_anywhere_with_the_same_name_and_different_identifiers_are_duplicates() {
        let text = guide(
            "party @dem \"Democratic\"\nparty @dems \"Democratic\"\nparty @dem \"Democratic\"\nparty @a \"X\"\nparty @a \"X\"\ncandidate @c \"C\"\n  party @green \"Democratic\"\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n",
        );
        assert_eq!(
            reported_as(&text, "duplicate-party-name"),
            [(82, 94), (106, 118), (177, 189)]
        );
    }

    #[test]
    fn a_top_level_declaration_no_party_field_or_member_resolves_to_is_unused() {
        let text = guide(
            "party @dem \"Democratic\"  # c\n  abbreviation \"D\"\nparty @gop \"Republican\"\ncandidate @a \"A\"\ncandidate @c \"C\"\n  party @dem\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @c\n    recommend\n      support @a\n",
        );
        assert_eq!(
            reported_as(&text, "unused-declaration"),
            [(94, 117), (118, 134)]
        );
    }

    #[test]
    fn a_mismatched_reference_uses_the_declaration_it_names() {
        let text = guide(
            "candidate @a \"A\"\ncandidate @c \"C\"\n  party @a\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @c\n    recommend no-position\n",
        );
        assert_eq!(reported_as(&text, "unused-declaration"), []);
    }

    #[test]
    fn a_duplicate_declaration_is_never_used() {
        let text = guide(
            "party @dem \"D\"\nparty @dem \"D2\"\ncandidate @c \"C\"\n  party @dem\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @c\n    recommend no-position\n",
        );
        assert_eq!(
            reported(&text),
            [
                ("unused-declaration", 61, 76),
                ("duplicate-declaration", 67, 71)
            ]
        );
    }

    #[test]
    fn identifiers_in_discarded_or_set_aside_text_or_without_a_word_are_not_resolved() {
        let text = guide(
            "candidate @c \"C\" @x\n  party @dem\n  party @gop\n  party @\nballot @b \"B\"\n  contest @k \"K\" single\n    campaign @c\n    recommend no-position\n",
        );
        assert_eq!(
            reported(&text),
            [
                ("unexpected-token", 63, 65),
                ("undeclared-reference", 74, 78),
                ("repeated-field", 81, 91),
                ("repeated-field", 94, 101),
                ("missing-identifier", 101, 101),
            ]
        );
    }
}
