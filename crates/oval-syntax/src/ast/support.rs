use rowan::NodeOrToken;

use super::{AstNode, AstToken};
use crate::{Keyword, SyntaxKind, SyntaxNode, SyntaxToken};

pub(super) fn child<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    children(node).next()
}

pub(super) fn children<N: AstNode>(node: &SyntaxNode) -> impl Iterator<Item = N> + use<N> {
    node.children().filter_map(N::cast)
}

pub(super) fn field<N: AstNode>(node: &SyntaxNode, keyword: Keyword) -> Option<N> {
    children(node).find(|field: &N| self::keyword(field.syntax()) == Some(keyword))
}

pub(super) fn block_children<N: AstNode>(node: &SyntaxNode) -> impl Iterator<Item = N> + use<N> {
    node.children()
        .find(|child| child.kind() == SyntaxKind::Block)
        .into_iter()
        .flat_map(|block| children(&block))
}

pub(super) fn block_field<N: AstNode>(node: &SyntaxNode, keyword: Keyword) -> Option<N> {
    block_children(node).find(|field: &N| self::keyword(field.syntax()) == Some(keyword))
}

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
    keyword_token(node, keyword).is_some()
}

pub(super) fn keyword_token(node: &SyntaxNode, keyword: Keyword) -> Option<SyntaxToken> {
    words(node).find(|word| word.text() == keyword.text())
}

fn words(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + use<> {
    child_tokens(node).filter(|token| token.kind() == SyntaxKind::Word)
}

fn child_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + use<> {
    node.children_with_tokens()
        .filter_map(NodeOrToken::into_token)
}
