//! Converts source text into a flat sequence of tokens.

use rowan::{TextRange, TextSize};

use crate::{Keyword, SyntaxKind};

/// An oval syntax token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// The kind of token.
    pub kind: SyntaxKind,

    /// The length of the token, in bytes.
    pub len: TextSize,
}

/// The reasons a lexical error can occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LexErrorReason {
    /// A horizontal tab outside a string or Markdown body.
    IllegalTab,

    /// An `@` not followed by a letter.
    MalformedIdentifier,

    /// A word that is not a keyword.
    UnknownWord,

    /// A character that cannot begin any token.
    UnexpectedCharacter,

    /// A string literal with no closing quote before the end of the line.
    UnterminatedString,

    /// A backslash in a string followed by something other than `"` or `\\`.
    InvalidEscape,

    /// A word beginning with a digit that is neither an integer nor a date.
    MalformedNumber,

    /// A URL scheme and `://` with nothing after them.
    MalformedUrl,
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
    /// Every [`SyntaxKind::Error`] token has exactly one error spanning it;
    /// other tokens may carry errors over part of their text.
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
                ':' => {
                    self.pos += 1;
                    self.push(SyntaxKind::Colon, start);
                }
                ',' => {
                    self.pos += 1;
                    self.push(SyntaxKind::Comma, start);
                }
                '@' => {
                    self.pos += 1;
                    let well_formed = self.peek().is_some_and(|c| c.is_ascii_alphabetic());
                    self.bump_while(is_word_char);
                    if well_formed {
                        self.push(SyntaxKind::Identifier, start);
                    } else {
                        self.push_error(LexErrorReason::MalformedIdentifier, start);
                    }
                }
                c if c.is_ascii_digit() => self.number(start),
                c if is_word_char(c) => self.word(start),
                '"' => self.string(start),
                c => {
                    self.pos += c.len_utf8();
                    self.push_error(LexErrorReason::UnexpectedCharacter, start);
                }
            }
        }
    }

    fn number(&mut self, start: usize) {
        self.bump_while(is_word_char);
        let text = &self.source[start..self.pos];
        if text.bytes().all(|b| b.is_ascii_digit()) {
            self.push(SyntaxKind::IntegerLiteral, start);
        } else if is_date_shaped(text) {
            self.push(SyntaxKind::DateLiteral, start);
        } else {
            self.push_error(LexErrorReason::MalformedNumber, start);
        }
    }

    fn word(&mut self, start: usize) {
        self.bump_while(is_word_char);
        let text = &self.source[start..self.pos];
        if matches!(text, "http" | "https") && self.rest().starts_with("://") {
            self.pos += "://".len();
            let path_start = self.pos;
            self.bump_while(|c| c != ' ' && c != '\t' && !is_line_terminator(c));
            if self.pos == path_start {
                self.push_error(LexErrorReason::MalformedUrl, start);
            } else {
                self.push(SyntaxKind::UrlLiteral, start);
            }
            return;
        }
        match Keyword::from_text(text) {
            Some(keyword) => self.push(keyword.into(), start),
            None => self.push_error(LexErrorReason::UnknownWord, start),
        }
    }

    fn string(&mut self, start: usize) {
        self.pos += 1;
        loop {
            match self.peek() {
                Some('"') => {
                    self.pos += 1;
                    self.push(SyntaxKind::StringLiteral, start);
                    return;
                }
                Some('\\') => {
                    let escape_start = self.pos;
                    self.pos += 1;
                    match self.peek() {
                        Some('"' | '\\') => self.pos += 1,
                        Some(c) if !is_line_terminator(c) => {
                            self.pos += c.len_utf8();
                            self.report(LexErrorReason::InvalidEscape, escape_start, self.pos);
                        }
                        _ => {}
                    }
                }
                Some(c) if !is_line_terminator(c) => self.pos += c.len_utf8(),
                _ => {
                    self.push_error(LexErrorReason::UnterminatedString, start);
                    return;
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

    fn report(&mut self, reason: LexErrorReason, start: usize, end: usize) {
        let range = TextRange::new(text_size(start), text_size(end));
        self.out.errors.push(LexError { reason, range });
    }

    fn push_error(&mut self, reason: LexErrorReason, start: usize) {
        self.report(reason, start, self.pos);
        self.push(SyntaxKind::Error, start);
    }
}

fn text_size(offset: usize) -> TextSize {
    TextSize::try_from(offset).expect("source longer than 4 GiB")
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
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| matches!(i, 4 | 7) || b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use rowan::TextRange;

    use super::{LexError, LexErrorReason, lex, text_size};
    use crate::Keyword;
    use crate::SyntaxKind::{
        self, Colon, Comma, Comment, DateLiteral, Error, Identifier, IntegerLiteral, Newline,
        StringLiteral, UrlLiteral, Whitespace,
    };

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
        let mut pos = 0;
        for token in &lexed.tokens {
            let end = pos + usize::from(token.len);
            if token.kind == Error {
                let spanning = lexed
                    .errors
                    .iter()
                    .filter(|e| e.range == TextRange::new(text_size(pos), text_size(end)))
                    .count();
                assert_eq!(spanning, 1, "Error token at {pos}..{end} in {source:?}");
            }
            pos = end;
        }
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
    fn every_keyword_lexes_to_its_kind() {
        for keyword in Keyword::ALL.iter().copied() {
            let errors = check(keyword.text(), &[(keyword.into(), keyword.text())]);
            assert_eq!(errors.len(), 0, "{keyword:?}");
        }
    }

    #[test]
    fn keywords_are_case_sensitive() {
        let errors = check("Guide", &[(Error, "Guide")]);
        assert_eq!(errors[0].reason, LexErrorReason::UnknownWord);
    }

    #[test]
    fn unknown_words_are_errors() {
        let errors = check(
            "foo bar",
            &[(Error, "foo"), (Whitespace, " "), (Error, "bar")],
        );
        assert_eq!(
            errors,
            [
                LexError {
                    reason: LexErrorReason::UnknownWord,
                    range: range(0, 3)
                },
                LexError {
                    reason: LexErrorReason::UnknownWord,
                    range: range(4, 7)
                },
            ]
        );
        check("no_position", &[(Error, "no_position")]);
        check("yes2", &[(Error, "yes2")]);
    }

    #[test]
    fn words_stop_at_comments_and_terminators() {
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
    fn identifiers() {
        check("@a", &[(Identifier, "@a")]);
        check("@a-b_c9", &[(Identifier, "@a-b_c9")]);
        check("@Abc", &[(Identifier, "@Abc")]);
        check(
            "@a @b",
            &[(Identifier, "@a"), (Whitespace, " "), (Identifier, "@b")],
        );
    }

    #[test]
    fn malformed_identifiers_are_one_error_each() {
        for source in ["@", "@9", "@-x", "@_x", "@9abc"] {
            let errors = check(source, &[(Error, source)]);
            assert_eq!(
                errors[0].reason,
                LexErrorReason::MalformedIdentifier,
                "{source:?}"
            );
            let end = u32::try_from(source.len()).unwrap();
            assert_eq!(errors[0].range, range(0, end), "{source:?}");
        }
    }

    #[test]
    fn punctuators() {
        check("note:", &[(Keyword::Note.into(), "note"), (Colon, ":")]);
        check(
            "yes,no",
            &[
                (Keyword::Yes.into(), "yes"),
                (Comma, ","),
                (Keyword::No.into(), "no"),
            ],
        );
        check(
            "@a,@b",
            &[(Identifier, "@a"), (Comma, ","), (Identifier, "@b")],
        );
        check("::", &[(Colon, ":"), (Colon, ":")]);
    }

    #[test]
    fn a_header_line() {
        check(
            "campaign incumbent @bsantos",
            &[
                (Keyword::Campaign.into(), "campaign"),
                (Whitespace, " "),
                (Keyword::Incumbent.into(), "incumbent"),
                (Whitespace, " "),
                (Identifier, "@bsantos"),
            ],
        );
    }

    #[test]
    fn unexpected_characters_are_one_error_each() {
        let errors = check("!", &[(Error, "!")]);
        assert_eq!(errors[0].reason, LexErrorReason::UnexpectedCharacter);
        check("!?", &[(Error, "!"), (Error, "?")]);
    }

    #[test]
    fn string_literals() {
        check("\"abc\"", &[(StringLiteral, "\"abc\"")]);
        check("\"\"", &[(StringLiteral, "\"\"")]);
        check("\"José Cisneros\"", &[(StringLiteral, "\"José Cisneros\"")]);
    }

    #[test]
    fn strings_may_contain_anything_but_quotes_and_terminators() {
        check(
            "\"# not a comment\"",
            &[(StringLiteral, "\"# not a comment\"")],
        );
        check(
            "\"@not-an-id, yes:\"",
            &[(StringLiteral, "\"@not-an-id, yes:\"")],
        );
        check("\"a\tb\"", &[(StringLiteral, "\"a\tb\"")]);
    }

    #[test]
    fn string_escapes() {
        let errors = check("\"a\\\"b\"", &[(StringLiteral, "\"a\\\"b\"")]);
        assert_eq!(errors.len(), 0, "{errors:?}");
        check("\"a\\\\b\"", &[(StringLiteral, "\"a\\\\b\"")]);
        check("\"\\\"\\\\\"", &[(StringLiteral, "\"\\\"\\\\\"")]);
    }

    #[test]
    fn invalid_escapes_keep_the_string_and_report_the_escape() {
        let errors = check("\"a\\nb\"", &[(StringLiteral, "\"a\\nb\"")]);
        assert_eq!(
            errors,
            [LexError {
                reason: LexErrorReason::InvalidEscape,
                range: range(2, 4)
            }]
        );
        let errors = check("\"\\x\\y\"", &[(StringLiteral, "\"\\x\\y\"")]);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert_eq!(errors[1].range, range(3, 5));
    }

    #[test]
    fn unterminated_strings_stop_at_the_line_end() {
        let errors = check("\"abc", &[(Error, "\"abc")]);
        assert_eq!(
            errors,
            [LexError {
                reason: LexErrorReason::UnterminatedString,
                range: range(0, 4)
            }]
        );
        check(
            "\"abc\n\"ok\"",
            &[(Error, "\"abc"), (Newline, "\n"), (StringLiteral, "\"ok\"")],
        );
        check("\"abc\r\n", &[(Error, "\"abc"), (Newline, "\r\n")]);
        check("\"", &[(Error, "\"")]);
    }

    #[test]
    fn trailing_backslash_is_only_unterminated() {
        let errors = check("\"abc\\", &[(Error, "\"abc\\")]);
        assert_eq!(
            errors,
            [LexError {
                reason: LexErrorReason::UnterminatedString,
                range: range(0, 5)
            }]
        );
        let errors = check("\"abc\\\n", &[(Error, "\"abc\\"), (Newline, "\n")]);
        assert_eq!(errors.len(), 1, "{errors:?}");
    }

    #[test]
    fn strings_in_context() {
        check(
            "supporters \"a\", \"b\"",
            &[
                (Keyword::Supporters.into(), "supporters"),
                (Whitespace, " "),
                (StringLiteral, "\"a\""),
                (Comma, ","),
                (Whitespace, " "),
                (StringLiteral, "\"b\""),
            ],
        );
        check(
            "designation\"x\"# c",
            &[
                (Keyword::Designation.into(), "designation"),
                (StringLiteral, "\"x\""),
                (Comment, "# c"),
            ],
        );
    }

    #[test]
    fn multibyte_characters_are_measured_in_bytes() {
        let errors = check("é ", &[(Error, "é"), (Whitespace, " ")]);
        assert_eq!(errors[0].range, range(0, 2));
    }

    #[test]
    fn integer_literals() {
        check("3", &[(IntegerLiteral, "3")]);
        check("007", &[(IntegerLiteral, "007")]);
        check(
            "3,4",
            &[(IntegerLiteral, "3"), (Comma, ","), (IntegerLiteral, "4")],
        );
        check("3#c", &[(IntegerLiteral, "3"), (Comment, "#c")]);
        check(
            "ranked max 10",
            &[
                (Keyword::Ranked.into(), "ranked"),
                (Whitespace, " "),
                (Keyword::Max.into(), "max"),
                (Whitespace, " "),
                (IntegerLiteral, "10"),
            ],
        );
    }

    #[test]
    fn date_literals_match_the_shape_only() {
        check("2026-11-05", &[(DateLiteral, "2026-11-05")]);
        check("0000-00-00", &[(DateLiteral, "0000-00-00")]);
        check("2026-13-45", &[(DateLiteral, "2026-13-45")]);
        check(
            "date 2026-11-05\n",
            &[
                (Keyword::Date.into(), "date"),
                (Whitespace, " "),
                (DateLiteral, "2026-11-05"),
                (Newline, "\n"),
            ],
        );
    }

    #[test]
    fn malformed_numbers_are_one_error_each() {
        for source in [
            "12abc",
            "2026-11-5",
            "2026-11-055",
            "3_000",
            "1-2",
            "2026-11-05x",
            "20261105-",
        ] {
            let errors = check(source, &[(Error, source)]);
            assert_eq!(
                errors[0].reason,
                LexErrorReason::MalformedNumber,
                "{source:?}"
            );
        }
    }

    #[test]
    fn url_literals_run_to_whitespace() {
        check(
            "https://leepatel2026.example",
            &[(UrlLiteral, "https://leepatel2026.example")],
        );
        check(
            "http://a.b/c?d=1&e=2#frag",
            &[(UrlLiteral, "http://a.b/c?d=1&e=2#frag")],
        );
        check(
            "https://x.y/\"quoted\"",
            &[(UrlLiteral, "https://x.y/\"quoted\"")],
        );
        check(
            "website https://x.y # c\n",
            &[
                (Keyword::Website.into(), "website"),
                (Whitespace, " "),
                (UrlLiteral, "https://x.y"),
                (Whitespace, " "),
                (Comment, "# c"),
                (Newline, "\n"),
            ],
        );
        check(
            "https://x.y\r\n",
            &[(UrlLiteral, "https://x.y"), (Newline, "\r\n")],
        );
    }

    #[test]
    fn url_requires_scheme_separator_and_body() {
        let errors = check("https://", &[(Error, "https://")]);
        assert_eq!(errors[0].reason, LexErrorReason::MalformedUrl);
        let errors = check(
            "https:// x",
            &[(Error, "https://"), (Whitespace, " "), (Error, "x")],
        );
        assert_eq!(errors[0].reason, LexErrorReason::MalformedUrl);
        let errors = check("https:", &[(Error, "https"), (Colon, ":")]);
        assert_eq!(errors[0].reason, LexErrorReason::UnknownWord);
        let errors = check(
            "ftp://x",
            &[
                (Error, "ftp"),
                (Colon, ":"),
                (Error, "/"),
                (Error, "/"),
                (Error, "x"),
            ],
        );
        assert_eq!(errors[0].reason, LexErrorReason::UnknownWord);
        assert_eq!(errors[1].reason, LexErrorReason::UnexpectedCharacter);
    }
}
