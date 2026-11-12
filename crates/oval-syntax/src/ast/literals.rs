mod date;
mod integer;
mod markdown_body;
mod string;
mod url;

pub use date::DateLiteral;
pub use integer::IntegerLiteral;
pub use markdown_body::MarkdownBody;
pub use string::StringLiteral;
pub use url::UrlLiteral;

#[cfg(test)]
fn tokens(text: &str) -> Vec<crate::SyntaxToken> {
    crate::parse(text)
        .unwrap()
        .syntax()
        .descendants_with_tokens()
        .filter_map(rowan::NodeOrToken::into_token)
        .collect()
}
