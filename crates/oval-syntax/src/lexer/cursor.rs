mod layout;
mod markdown;

use super::to_u32;
use crate::{LexResult, LexicalError, LexicalErrorKind, Token, TokenKind};

const BYTE_ORDER_MARK: char = '\u{FEFF}';

pub(super) struct Cursor<'a> {
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

impl<'a> Cursor<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        Cursor {
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

    fn byte_order_mark(&mut self, start: usize) {
        self.bump(BYTE_ORDER_MARK);
        self.push(TokenKind::ByteOrderMark, start);
        // The first line's indentation is counted after the byte order mark.
        self.line_start = self.pos;
    }

    fn whitespace(&mut self, start: usize) {
        self.eat_while(|c| c == ' ');
        self.push(TokenKind::Whitespace, start);
    }

    fn illegal_tab(&mut self, start: usize) {
        self.eat_while(|c| c == '\t');
        self.push_error(LexicalErrorKind::IllegalTab, start);
    }

    fn comment(&mut self, start: usize) {
        self.eat_while(|c| !is_line_terminator(c));
        self.push(TokenKind::Comment, start);
    }

    fn line_terminator(&mut self, c: char, start: usize) {
        self.bump(c);
        if c == '\r' && self.peek() == Some('\n') {
            self.bump('\n');
        }
        let kind = if self.line_has_tokens {
            self.body_opener = self.line_ends_with_colon.then_some(self.line_indentation);
            TokenKind::Newline
        } else {
            TokenKind::BlankLine
        };
        self.push(kind, start);
        self.line_has_tokens = false;
        self.line_start = self.pos;
    }

    fn at_sign(&mut self, start: usize) {
        self.single(TokenKind::AtSign, '@', start);
    }

    fn colon(&mut self, start: usize) {
        self.single(TokenKind::Colon, ':', start);
    }

    fn comma(&mut self, start: usize) {
        self.single(TokenKind::Comma, ',', start);
    }

    fn string(&mut self, start: usize) {
        self.bump('"');
        let mut invalid_escapes = Vec::new();
        let terminated = loop {
            match self.peek() {
                Some('"') => {
                    self.bump('"');
                    break true;
                }
                Some('\\') => {
                    let escape_start = self.pos;
                    self.bump('\\');
                    if let Some(c) = self.peek()
                        && !is_line_terminator(c)
                    {
                        self.bump(c);
                        if !matches!(c, '"' | '\\') {
                            invalid_escapes.push((escape_start, self.pos));
                        }
                    }
                }
                Some(c) if !is_line_terminator(c) => self.bump(c),
                _ => break false,
            }
        };
        if terminated {
            for (escape_start, escape_end) in invalid_escapes {
                self.report(LexicalErrorKind::InvalidEscape, escape_start, escape_end);
            }
            self.push(TokenKind::String, start);
        } else {
            self.push_error(LexicalErrorKind::UnterminatedString, start);
        }
    }

    fn number(&mut self, start: usize) {
        self.eat_while(|c| is_word_char(c) || c == '.');
        let text = &self.source[start..self.pos];
        if text.bytes().all(|b| b.is_ascii_digit()) {
            self.push(TokenKind::Integer, start);
        } else if is_date_shaped(text) {
            self.push(TokenKind::Date, start);
        } else {
            self.push_error(LexicalErrorKind::MalformedNumber, start);
        }
    }

    fn word(&mut self, start: usize) {
        self.eat_while(is_word_char);
        self.push(TokenKind::Word, start);
    }

    fn at_url(&self) -> bool {
        self.rest().starts_with("http://") || self.rest().starts_with("https://")
    }

    fn url(&mut self, start: usize) {
        self.eat_while(|c| c != ':');
        self.pos += "://".len();
        let path_start = self.pos;
        self.eat_while(|c| !matches!(c, ' ' | '\t') && !is_line_terminator(c));
        if self.pos == path_start {
            self.push_error(LexicalErrorKind::MalformedUrl, start);
        } else {
            self.push(TokenKind::Url, start);
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
