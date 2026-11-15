use oval_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

pub(crate) struct Entry {
    pub(crate) line: Vec<SyntaxToken>,
    pub(crate) body: Option<SyntaxToken>,
    pub(crate) children: Vec<Entry>,
}

pub(crate) fn entries(parent: &SyntaxNode) -> Vec<Entry> {
    parent.children().map(|node| entry(&node)).collect()
}

fn entry(node: &SyntaxNode) -> Entry {
    let mut entry = Entry {
        line: Vec::new(),
        body: None,
        children: Vec::new(),
    };
    collect(node, &mut entry);
    entry
}

fn collect(node: &SyntaxNode, entry: &mut Entry) {
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::MarkdownBlock => {
                entry.body = Some(token);
            }
            SyntaxElement::Token(token) => {
                if is_significant(token.kind()) {
                    entry.line.push(token);
                }
            }
            SyntaxElement::Node(block) if block.kind() == SyntaxKind::Block => {
                entry.children.extend(entries(&block));
            }
            SyntaxElement::Node(child) => collect(&child, entry),
        }
    }
}

fn is_significant(kind: SyntaxKind) -> bool {
    kind.token_kind()
        .is_some_and(|kind| !kind.is_ignored() && !kind.is_layout())
}
