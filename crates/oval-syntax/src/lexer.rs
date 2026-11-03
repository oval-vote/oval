//! Converts source text into a flat sequence of tokens.

use rowan::{TextRange, TextSize};

use crate::SyntaxKind;

/// An oval syntax token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// The kind of token.
    pub kind: SyntaxKind,

    /// The length of the token, in bytes.
    pub len: TextSize,
}

/// An enumeration of the kinds of lexical errors that can occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LexErrorReason {
    /// A horizontal tab outside a string or Markdown body.
    IllegalTab,
    /// A character that cannot begin any token.
    UnexpectedCharacter,
}

/// A lexical error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LexError {
    /// The kind of error
    pub reason: LexErrorReason,

    /// Where it went wrong.
    pub range: TextRange,
}

/// The result of lexing a source string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LexResult {
    /// The tokens, in source order.
    /// Their lengths sum to the length of the input.
    pub tokens: Vec<Token>,

    /// Errors, in source order.
    /// Each corresponds to an [`SyntaxKind::Error`] token.
    pub errors: Vec<LexError>,
}

/// Lexes the given source text into a flat sequence of tokens.
pub fn lex(source: &str) -> LexResult {
    let mut lexer = Lexer {
        source,
        pos: 0,
        out: LexResult::default(),
    };
    lexer.run();
    lexer.out
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    out: LexResult,
}

const BOM: char = '\u{FEFF}';

impl Lexer<'_> {
    fn run(&mut self) {
        if self.rest().starts_with(BOM) {
            let start = self.pos;
            self.pos += BOM.len_utf8();
            self.push(SyntaxKind::Whitespace, start);
        }
        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                '\r' => {
                    self.pos += 1;
                    if self.peek() == Some('\n') {
                        self.pos += 1;
                    }
                    self.push(SyntaxKind::Newline, start);
                }
                '\n' => {
                    self.pos += 1;
                    self.push(SyntaxKind::Newline, start);
                }
                ' ' => {
                    self.bump_while(|c| c == ' ');
                    self.push(SyntaxKind::Whitespace, start);
                }
                '#' => {
                    self.bump_while(|c| !is_line_terminator(c));
                    self.push(SyntaxKind::Comment, start);
                }
                '\t' => {
                    self.bump_while(|c| c == '\t');
                    self.push_error(LexErrorReason::IllegalTab, start);
                }
                _ => {
                    self.bump_while(|c| !is_token_boundary(c));
                    self.push_error(LexErrorReason::UnexpectedCharacter, start);
                }
            }
        }
    }

    fn rest(&self) -> &str {
        &self.source[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump_while(&mut self, pred: impl Fn(char) -> bool) {
        let consumed: usize = self
            .rest()
            .chars()
            .take_while(|c| pred(*c))
            .map(char::len_utf8)
            .sum();
        self.pos += consumed;
    }

    fn push(&mut self, kind: SyntaxKind, start: usize) {
        let len = TextSize::try_from(self.pos - start).expect("token longer than 4 GiB");
        self.out.tokens.push(Token { kind, len });
    }

    fn push_error(&mut self, reason: LexErrorReason, start: usize) {
        let range = TextRange::new(text_size(start), text_size(self.pos));
        self.out.errors.push(LexError { reason, range });
        self.push(SyntaxKind::Error, start);
    }
}

fn text_size(offset: usize) -> TextSize {
    TextSize::try_from(offset).expect("source longer than 4 GiB")
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r')
}

fn is_token_boundary(c: char) -> bool {
    matches!(c, ' ' | '\t' | '#') || is_line_terminator(c)
}

#[cfg(test)]
mod tests {
    use rowan::TextRange;

    use super::{LexError, LexErrorReason, lex};
    use crate::SyntaxKind::{self, Comment, Error, Newline, Whitespace};

    fn check(source: &str, expected: &[(SyntaxKind, &str)]) -> Vec<LexError> {
        let lexed = lex(source);
        let mut pos = 0;
        let mut actual = Vec::new();
        for token in &lexed.tokens {
            let len = usize::from(token.len);
            actual.push((token.kind, &source[pos..pos + len]));
            pos += len;
        }
        assert_eq!(
            pos,
            source.len(),
            "tokens do not cover the input {source:?}"
        );
        assert_eq!(actual, expected, "token stream for {source:?}");
        let error_tokens = lexed.tokens.iter().filter(|t| t.kind == Error).count();
        assert_eq!(
            lexed.errors.len(),
            error_tokens,
            "one error per Error token in {source:?}"
        );
        lexed.errors
    }

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    #[test]
    fn empty_input() {
        let errors = check("", &[]);
        assert_eq!(errors.len(), 0, "unexpected errors {errors:?}");
    }

    #[test]
    fn whitespace_runs_are_one_token() {
        check("   ", &[(Whitespace, "   ")]);
    }

    #[test]
    fn every_line_terminator_form_is_a_newline() {
        check("\n", &[(Newline, "\n")]);
        check("\r\n", &[(Newline, "\r\n")]);
        check("\r", &[(Newline, "\r")]);
        check(
            "\n\r\n\r",
            &[(Newline, "\n"), (Newline, "\r\n"), (Newline, "\r")],
        );
    }

    #[test]
    fn blank_lines_lex_as_newlines() {
        check(
            "\n  \n",
            &[(Newline, "\n"), (Whitespace, "  "), (Newline, "\n")],
        );
    }

    #[test]
    fn comment_runs_to_end_of_line() {
        check(
            "# a comment\n",
            &[(Comment, "# a comment"), (Newline, "\n")],
        );
        check("# no newline", &[(Comment, "# no newline")]);
        check(
            "  # indented",
            &[(Whitespace, "  "), (Comment, "# indented")],
        );
        check("#", &[(Comment, "#")]);
    }

    #[test]
    fn comment_may_contain_anything_but_a_terminator() {
        check(
            "# tabs\tand \"quotes\" @ids\r\n",
            &[(Comment, "# tabs\tand \"quotes\" @ids"), (Newline, "\r\n")],
        );
    }

    #[test]
    fn byte_order_mark_is_whitespace() {
        check("\u{FEFF}# x", &[(Whitespace, "\u{FEFF}"), (Comment, "# x")]);
        check("\u{FEFF}", &[(Whitespace, "\u{FEFF}")]);
    }

    #[test]
    fn tab_is_an_error() {
        let errors = check("\t\t# c", &[(Error, "\t\t"), (Comment, "# c")]);
        assert_eq!(
            errors,
            [LexError {
                reason: LexErrorReason::IllegalTab,
                range: range(0, 2)
            }]
        );
    }

    #[test]
    fn unexpected_text_is_one_error_per_run() {
        let errors = check(
            "foo bar",
            &[(Error, "foo"), (Whitespace, " "), (Error, "bar")],
        );
        assert_eq!(
            errors,
            [
                LexError {
                    reason: LexErrorReason::UnexpectedCharacter,
                    range: range(0, 3)
                },
                LexError {
                    reason: LexErrorReason::UnexpectedCharacter,
                    range: range(4, 7)
                },
            ]
        );
    }

    #[test]
    fn unexpected_runs_stop_at_comments_and_terminators() {
        check(
            "foo#bar\nbaz",
            &[
                (Error, "foo"),
                (Comment, "#bar"),
                (Newline, "\n"),
                (Error, "baz"),
            ],
        );
    }

    #[test]
    fn multibyte_characters_are_measured_in_bytes() {
        let errors = check("é ", &[(Error, "é"), (Whitespace, " ")]);
        assert_eq!(errors[0].range, range(0, 2));
    }
}
