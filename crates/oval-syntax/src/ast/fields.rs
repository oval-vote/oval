mod date_field;
mod markdown_field;
mod string_field;
mod string_list_field;
mod url_field;

pub use date_field::DateField;
pub use markdown_field::MarkdownField;
pub use string_field::StringField;
pub use string_list_field::StringListField;
pub use url_field::UrlField;

#[cfg(test)]
fn first<N: crate::ast::AstNode>(text: &str) -> N {
    crate::parse(text)
        .unwrap()
        .syntax()
        .descendants()
        .find_map(N::cast)
        .expect("the text has a node of this kind")
}
