use crate::{Token, TokenKind};

mod input_rejection;
mod layout;
mod lex_result;
mod lexical_error;

pub use input_rejection::InputRejection;
pub use lex_result::LexResult;
pub use lexical_error::{LexicalError, LexicalErrorKind};

const BYTE_ORDER_MARK: char = '\u{FEFF}';

/// Splits `text` into tokens.
///
/// Every byte of `text` belongs to exactly one token, so the lengths of the returned tokens always
/// add up to `text.len()`.
///
/// So far, only a few tokens are recognized. A byte order mark (U+FEFF) at the very start of `text`
/// becomes a [`TokenKind::ByteOrderMark`] token, each run of spaces becomes a
/// [`TokenKind::Whitespace`] token, and each `#` up to the end of its line becomes a
/// [`TokenKind::Comment`] token. Each line terminator, which is a line feed (U+000A), a carriage
/// return (U+000D) followed by a line feed, or a carriage return alone, becomes one
/// [`TokenKind::Newline`] token when its line has other tokens, and one [`TokenKind::BlankLine`]
/// token when the line contains only whitespace, a comment, or a byte order mark. Each `@`, `:`,
/// and `,` becomes a [`TokenKind::AtSign`], [`TokenKind::Colon`], or [`TokenKind::Comma`] token, and
/// each ASCII letter followed by any ASCII letters, digits, `-`, and `_` becomes a
/// [`TokenKind::Word`] token. Each `"` up to its closing `"` becomes a [`TokenKind::String`]
/// token, where `\"` and `\\` do not close it; a string with no closing `"` before the end of its
/// line becomes a single [`TokenKind::Error`] token instead. Each run of tabs outside a string or a
/// comment becomes a single [`TokenKind::Error`] token. Each run of ASCII letters, digits, `-`, `_`,
/// and `.` that begins with a digit becomes a [`TokenKind::Integer`] token if it is all digits, a
/// [`TokenKind::Date`] token if it has the shape `YYYY-MM-DD`, and a single [`TokenKind::Error`]
/// token otherwise. Each `http://` or `https://` followed by anything up to the next space, tab, or
/// line terminator becomes a [`TokenKind::Url`] token, and a single [`TokenKind::Error`] token if
/// nothing follows. Every other character becomes its own [`TokenKind::Error`] token, which is how
/// the specification treats a character that cannot begin any token.
///
/// Every line with tokens ends in a [`TokenKind::Newline`]. When the last line has no line
/// terminator, its `Newline` is zero-length.
///
/// Lexing never stops at an error. Each [`TokenKind::Error`] token has a [`LexicalError`] in
/// [`LexResult::errors`] spanning exactly its bytes, and lexing continues after it.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `text` contains U+0000, which the specification rejects as a
/// whole, or is longer than [`u32::MAX`] bytes. No tokens are produced.
pub fn lex(text: &str) -> Result<LexResult, InputRejection> {
    to_offset(text.len())?;
    if let Some(offset) = text.find('\0') {
        return Err(InputRejection::NulCharacter {
            offset: to_offset(offset)?,
        });
    }
    let mut lexer = Lexer {
        source: text,
        pos: 0,
        tokens: Vec::new(),
        errors: Vec::new(),
        line_has_tokens: false,
    };
    lexer.run();
    Ok(LexResult {
        tokens: lexer.tokens,
        errors: lexer.errors,
    })
}

/// Splits a document given as raw bytes into tokens.
///
/// The bytes are decoded as UTF-8 and then lexed by [`lex`]. Use this where a document arrives as
/// bytes, such as when reading a file, so that malformed UTF-8 is rejected rather than repaired.
///
/// # Errors
///
/// Returns an [`InputRejection`] if `bytes` is not well-formed UTF-8 or contains U+0000, which the
/// specification rejects as a whole, or is longer than [`u32::MAX`] bytes. No tokens are produced.
pub fn lex_bytes(bytes: &[u8]) -> Result<LexResult, InputRejection> {
    to_offset(bytes.len())?;
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            return Err(InputRejection::InvalidUtf8 {
                offset: to_offset(error.valid_up_to())?,
            });
        }
    };
    lex(text)
}

fn to_offset(n: usize) -> Result<u32, InputRejection> {
    u32::try_from(n).map_err(|_| InputRejection::TooLarge)
}

// Cannot fail: `lex` rejects any text longer than u32::MAX bytes, so every position and length
// within it fits.
fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("lex rejects text longer than u32::MAX bytes")
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<LexicalError>,
    line_has_tokens: bool,
}

impl Lexer<'_> {
    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let start = self.pos;
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
    }

    fn byte_order_mark(&mut self, start: usize) {
        self.bump(BYTE_ORDER_MARK);
        self.push(TokenKind::ByteOrderMark, start);
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
            TokenKind::Newline
        } else {
            TokenKind::BlankLine
        };
        self.push(kind, start);
        self.line_has_tokens = false;
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
mod tests;
