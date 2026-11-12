use rowan::NodeOrToken;

use super::AstToken;
use crate::{Keyword, SyntaxKind, SyntaxNode, SyntaxToken};

pub(super) fn token<T: AstToken>(node: &SyntaxNode) -> Option<T> {
    tokens(node).next()
}

pub(super) fn tokens<T: AstToken>(node: &SyntaxNode) -> impl Iterator<Item = T> + use<T> {
    child_tokens(node).filter_map(T::cast)
}

pub(super) fn keyword(node: &SyntaxNode) -> Option<Keyword> {
    words(node)
        .next()
        .and_then(|word| Keyword::from_text(word.text()))
}

pub(super) fn has_keyword(node: &SyntaxNode, keyword: Keyword) -> bool {
    words(node).any(|word| word.text() == keyword.text())
}

fn words(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + use<> {
    child_tokens(node).filter(|token| token.kind() == SyntaxKind::Word)
}

fn child_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + use<> {
    node.children_with_tokens()
        .filter_map(NodeOrToken::into_token)
}
