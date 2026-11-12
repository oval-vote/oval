use crate::ast::ast_token;

ast_token! {
    /// A URL, such as `https://dems.example`, written without quotes.
    ///
    /// Its value is its [`text`](crate::ast::AstToken::text), from the `http` or `https` scheme to
    /// the next space or the end of the line.
    UrlLiteral(Url)
}

#[cfg(test)]
mod tests {
    use super::UrlLiteral;
    use crate::SyntaxKind;
    use crate::ast::AstToken;
    use crate::ast::literals::tokens;

    #[test]
    fn a_url_is_its_text() {
        let url = tokens("website https://dems.example/join?ref=guide#top\n")
            .into_iter()
            .find_map(UrlLiteral::cast)
            .expect("the website field has a URL");
        assert_eq!(url.text(), "https://dems.example/join?ref=guide#top");
    }

    #[test]
    fn only_a_url_token_is_a_url_literal() {
        assert!(UrlLiteral::can_cast(SyntaxKind::Url));
        assert!(!UrlLiteral::can_cast(SyntaxKind::String));
        let word = tokens("website https://dems.example\n")
            .into_iter()
            .find(|token| token.kind() == SyntaxKind::Word)
            .expect("the field has a keyword");
        assert_eq!(UrlLiteral::cast(word), None);
    }
}
