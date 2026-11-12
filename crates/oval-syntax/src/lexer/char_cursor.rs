mod ignored;
mod layout;
mod markdown;
mod number;
mod punctuator;
mod string;
mod url;
mod word;

use super::to_u32;
use crate::{LexResult, LexicalError, LexicalErrorKind, Token, TokenKind};

const BYTE_ORDER_MARK: char = '\u{FEFF}';

pub(super) struct CharCursor<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<LexicalError>,
    line_has_tokens: bool,
    line_start: usize,
    indent_stack: Vec<usize>,
    line_ends_with_colon: bool,
    line_indentation: usize,
    body_opener: Option<usize>,
}

impl<'a> CharCursor<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        CharCursor {
            source,
            pos: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
            line_has_tokens: false,
            line_start: 0,
            indent_stack: vec![0],
            line_ends_with_colon: false,
            line_indentation: 0,
            body_opener: None,
        }
    }

    pub(super) fn run(mut self) -> LexResult {
        while let Some(c) = self.peek() {
            let start = self.pos;
            if start == self.line_start && self.markdown_block(start) {
                continue;
            }
            if !self.line_has_tokens && !self.starts_ignored_token(c) {
                self.begin_line(start);
            }
            match c {
                BYTE_ORDER_MARK if start == 0 => self.byte_order_mark(start),
                ' ' => self.whitespace(start),
                '\t' => self.illegal_tab(start),
                '#' => self.comment(start),
                '\n' | '\r' => self.line_terminator(c, start),
                '@' => self.at_sign(start),
                ':' => self.colon(start),
                ',' => self.comma(start),
                '"' => self.string(start),
                'h' if self.at_url() => self.url(start),
                c if c.is_ascii_digit() => self.number(start),
                c if c.is_ascii_alphabetic() => self.word(start),
                _ => self.unexpected_character(c, start),
            }
        }
        self.end_input();
        LexResult {
            tokens: self.tokens,
            errors: self.errors,
        }
    }

    fn unexpected_character(&mut self, c: char, start: usize) {
        self.bump(c);
        self.push_error(LexicalErrorKind::UnexpectedCharacter, start);
    }

    fn single(&mut self, kind: TokenKind, c: char, start: usize) {
        self.bump(c);
        self.push(kind, start);
    }

    fn rest(&self) -> &str {
        &self.source[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self, c: char) {
        self.pos += c.len_utf8();
    }

    fn eat_while(&mut self, predicate: impl Fn(char) -> bool) {
        while let Some(c) = self.peek() {
            if !predicate(c) {
                break;
            }
            self.bump(c);
        }
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        if !kind.is_ignored() {
            self.line_has_tokens = true;
            self.line_ends_with_colon = kind == TokenKind::Colon;
        }
        let len = to_u32(self.pos - start);
        self.tokens.push(Token { kind, len });
    }

    fn report(&mut self, kind: LexicalErrorKind, start: usize, end: usize) {
        let error = LexicalError {
            kind,
            offset: to_u32(start),
            len: to_u32(end - start),
        };
        self.errors.push(error);
    }

    fn push_error(&mut self, kind: LexicalErrorKind, start: usize) {
        self.report(kind, start, self.pos);
        self.push(TokenKind::Error, start);
    }
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r')
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
}

fn is_date_shaped(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, &b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
}

#[cfg(test)]
mod tests {
    use crate::LexicalErrorKind::UnexpectedCharacter;
    use crate::TokenKind::{Error, Newline};
    use crate::lexer::tests::{check_lexer, error};

    #[test]
    fn each_unexpected_character_is_its_own_error_token() {
        // "é" is one character but two bytes in UTF-8.
        check_lexer(
            "-é_",
            &[(Error, "-"), (Error, "é"), (Error, "_"), (Newline, "")],
            &[
                error(UnexpectedCharacter, 0, 1),
                error(UnexpectedCharacter, 1, 2),
                error(UnexpectedCharacter, 3, 1),
            ],
        );
    }
}
