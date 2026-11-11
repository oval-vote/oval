use crate::{Token, TokenKind};

/// Splits `text` into tokens.
///
/// Every byte of `text` belongs to exactly one token, so the lengths of the returned tokens always
/// add up to `text.len()`. Nothing is recognized yet: each character becomes its own
/// [`TokenKind::Error`] token, which is how the specification treats a character that cannot begin
/// any token.
pub fn lex(text: &str) -> Vec<Token> {
    let mut lexer = Lexer {
        source: text,
        pos: 0,
        tokens: Vec::new(),
    };
    lexer.run();
    lexer.tokens
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn run(&mut self) {
        while let Some(c) = self.peek() {
            self.bump(c);
            self.push(TokenKind::Error, c.len_utf8());
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn bump(&mut self, c: char) {
        self.pos += c.len_utf8();
    }

    fn push(&mut self, kind: TokenKind, len: usize) {
        let len = u32::try_from(len).expect("a token is at most u32::MAX bytes");
        self.tokens.push(Token { kind, len });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(kind: TokenKind, len: u32) -> Token {
        Token { kind, len }
    }

    #[test]
    fn empty_input_has_no_tokens() {
        assert_eq!(lex(""), vec![]);
    }

    #[test]
    fn each_unexpected_character_is_its_own_error_token() {
        // "é" is one character but two bytes in UTF-8.
        assert_eq!(
            lex("-é_"),
            vec![
                token(TokenKind::Error, 1),
                token(TokenKind::Error, 2),
                token(TokenKind::Error, 1),
            ]
        );
    }

    #[test]
    fn token_lengths_cover_the_input() {
        for text in [
            "",
            "-é_",
            "ballot @city \"City\"\n",
            "  # comment\r\n",
            "日本語",
        ] {
            let total: usize = lex(text).iter().map(|token| token.len as usize).sum();
            assert_eq!(total, text.len(), "input: {text:?}");
        }
    }
}
