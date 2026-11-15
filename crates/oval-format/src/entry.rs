use oval_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

pub(crate) struct Document {
    pub(crate) entries: Vec<Entry>,
    pub(crate) trailing: Vec<Trivia>,
}

#[derive(Default)]
pub(crate) struct Entry {
    pub(crate) leading: Vec<Trivia>,
    pub(crate) line: Vec<SyntaxToken>,
    pub(crate) comment: Option<String>,
    pub(crate) body: Option<SyntaxToken>,
    pub(crate) children: Vec<Entry>,
}

#[derive(PartialEq, Eq)]
pub(crate) enum Trivia {
    Blank,
    Comment(String),
}

pub(crate) fn document(root: &SyntaxNode) -> Document {
    let mut builder = Builder::default();
    let mut entries = builder.entries(root);
    let mut trailing = builder.pending;
    let start = match entries.first_mut() {
        Some(first) => &mut first.leading,
        None => &mut trailing,
    };
    if start.first() == Some(&Trivia::Blank) {
        start.remove(0);
    }
    if trailing.last() == Some(&Trivia::Blank) {
        trailing.pop();
    }
    Document { entries, trailing }
}

#[derive(Default)]
struct Builder {
    pending: Vec<Trivia>,
}

impl Builder {
    fn entries(&mut self, parent: &SyntaxNode) -> Vec<Entry> {
        let mut entries = Vec::new();
        for element in parent.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    let mut entry = Entry::default();
                    self.collect(&node, &mut entry);
                    entries.push(entry);
                }
                SyntaxElement::Token(token) => self.trivia(&token),
            }
        }
        entries
    }

    fn collect(&mut self, node: &SyntaxNode, entry: &mut Entry) {
        for element in node.children_with_tokens() {
            match element {
                SyntaxElement::Token(token) if token.kind() == SyntaxKind::MarkdownBlock => {
                    entry.body = Some(token);
                }
                SyntaxElement::Token(token) if is_trailing_comment(&token) => {
                    entry.comment = Some(comment(&token));
                }
                SyntaxElement::Token(token) if is_significant(token.kind()) => {
                    if entry.line.is_empty() {
                        entry.leading = std::mem::take(&mut self.pending);
                    }
                    entry.line.push(token);
                }
                SyntaxElement::Token(token) => self.trivia(&token),
                SyntaxElement::Node(block) if block.kind() == SyntaxKind::Block => {
                    entry.children = self.entries(&block);
                }
                SyntaxElement::Node(child) => self.collect(&child, entry),
            }
        }
    }

    fn trivia(&mut self, token: &SyntaxToken) {
        match token.kind() {
            SyntaxKind::Comment => self.pending.push(Trivia::Comment(comment(token))),
            SyntaxKind::BlankLine
                if !ends_comment_line(token) && self.pending.last() != Some(&Trivia::Blank) =>
            {
                self.pending.push(Trivia::Blank);
            }
            _ => {}
        }
    }
}

fn is_significant(kind: SyntaxKind) -> bool {
    kind.token_kind()
        .is_some_and(|kind| !kind.is_ignored() && !kind.is_layout())
}

fn is_trailing_comment(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::Comment
        && beyond_whitespace(token.next_token(), SyntaxToken::next_token)
            .is_some_and(|next| next.kind() == SyntaxKind::Newline)
}

fn ends_comment_line(blank: &SyntaxToken) -> bool {
    beyond_whitespace(blank.prev_token(), SyntaxToken::prev_token)
        .is_some_and(|previous| previous.kind() == SyntaxKind::Comment)
}

fn beyond_whitespace(
    start: Option<SyntaxToken>,
    step: fn(&SyntaxToken) -> Option<SyntaxToken>,
) -> Option<SyntaxToken> {
    std::iter::successors(start, step).find(|token| token.kind() != SyntaxKind::Whitespace)
}

fn comment(token: &SyntaxToken) -> String {
    token.text().trim_end_matches(' ').to_owned()
}

#[cfg(test)]
mod tests {
    use crate::format;

    fn formatted(text: &str) -> String {
        format(text).unwrap()
    }

    #[test]
    fn a_blank_line_and_the_comments_after_it_go_with_the_next_line_at_its_depth() {
        assert_eq!(
            formatted(
                "oval 1\n\n      # about the ballot\nballot @b \"B\"\n# inside, before the block\n  contest @k \"K\" single\n    campaign @a \"A\"\n\n    # before the dedent\n# also\n  measure @m \"M\"\n    recommend yes\n"
            ),
            "oval 1\n\n# about the ballot\nballot @b \"B\"\n  # inside, before the block\n  contest @k \"K\" single\n    campaign @a \"A\"\n\n  # before the dedent\n  # also\n  measure @m \"M\"\n    recommend yes\n"
        );
    }

    #[test]
    fn runs_of_blank_lines_collapse_to_one() {
        assert_eq!(
            formatted("oval 1\n\n\n   \nguide \"G\"\n\n# one\n\n\n# two\n\n\nelection \"E\"\n"),
            "oval 1\n\nguide \"G\"\n\n# one\n\n# two\n\nelection \"E\"\n"
        );
    }

    #[test]
    fn a_blank_line_at_the_start_of_a_block_stays() {
        let text =
            "oval 1\nballot @b \"B\"\n\n  measure @m \"M\"\n\n    # first\n    recommend yes\n";
        assert_eq!(formatted(text), text);
    }

    #[test]
    fn blank_lines_at_the_ends_of_the_document_are_removed_and_comments_after_it_start_at_column_zero()
     {
        assert_eq!(
            formatted(
                "\n\n# head\n\noval 1\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n\n\n    # tail\n\n  # end\n\n\n"
            ),
            "# head\n\noval 1\nballot @b \"B\"\n  measure @m \"M\"\n    recommend yes\n\n# tail\n\n# end\n"
        );
        assert_eq!(formatted("\n\noval 1\n\n"), "oval 1\n");
    }

    #[test]
    fn a_trailing_comment_follows_one_space_and_loses_its_trailing_spaces() {
        assert_eq!(
            formatted(
                "oval 1    # version   \nballot @b \"B\"  # b\n  measure @m \"M\"\n    note:   # n\n      Text.\n    recommend yes#y\n"
            ),
            "oval 1 # version\nballot @b \"B\" # b\n  measure @m \"M\"\n    note: # n\n      Text.\n    recommend yes #y\n"
        );
    }
}
