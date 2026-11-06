//! Converts source text into a flat sequence of tokens.

use std::fmt;

use rowan::{TextRange, TextSize};

use crate::SyntaxKind;

/// An oval syntax token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// The kind of token.
    pub kind: SyntaxKind,

    /// The length of the token, in bytes.
    pub len: TextSize,
}

/// The lexical errors of the specification, Section 3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LexicalErrorKind {
    /// A horizontal tab outside a string, a comment, or a Markdown body.
    IllegalTab,

    /// A character that cannot begin any token.
    UnexpectedCharacter,

    /// A string literal with no closing quote before the end of the line.
    UnterminatedString,

    /// A backslash in a string followed by something other than `"` or `\\`.
    InvalidEscape,

    /// A run beginning with a digit that is neither an integer nor a date.
    MalformedNumber,

    /// A URL scheme and `://` with nothing after them.
    MalformedUrl,

    /// A line indented less than the enclosing block but matching no open level.
    InconsistentDedent,
}

impl LexicalErrorKind {
    /// Every kind, in the specification's order.
    pub const ALL: &'static [Self] = &[
        Self::IllegalTab,
        Self::UnexpectedCharacter,
        Self::UnterminatedString,
        Self::InvalidEscape,
        Self::MalformedNumber,
        Self::MalformedUrl,
        Self::InconsistentDedent,
    ];

    /// The identifier the specification gives this error, such as `illegal-tab`.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::IllegalTab => "illegal-tab",
            Self::UnexpectedCharacter => "unexpected-character",
            Self::UnterminatedString => "unterminated-string",
            Self::InvalidEscape => "invalid-escape",
            Self::MalformedNumber => "malformed-number",
            Self::MalformedUrl => "malformed-url",
            Self::InconsistentDedent => "inconsistent-dedent",
        }
    }

    /// The title of this error's entry in the specification.
    pub const fn title(self) -> &'static str {
        match self {
            Self::IllegalTab => "Illegal Tab",
            Self::UnexpectedCharacter => "Unexpected Character",
            Self::UnterminatedString => "Unterminated String",
            Self::InvalidEscape => "Invalid Escape",
            Self::MalformedNumber => "Malformed Number",
            Self::MalformedUrl => "Malformed URL",
            Self::InconsistentDedent => "Inconsistent Dedent",
        }
    }
}

/// A lexical error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LexicalError {
    /// Which error it is.
    pub kind: LexicalErrorKind,

    /// Where it went wrong.
    pub range: TextRange,
}

/// Input that is rejected as a whole and produces no tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputRejection {
    /// The offset of the first U+0000 in the input.
    pub offset: TextSize,
}

impl fmt::Display for InputRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the input contains U+0000 at byte {} and is not an Oval document",
            u32::from(self.offset)
        )
    }
}

impl std::error::Error for InputRejection {}

/// The result of lexing a source string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LexResult {
    /// The tokens, in source order.
    /// Their lengths sum to the length of the input.
    pub tokens: Vec<Token>,

    /// Errors, in source order.
    /// Every [`SyntaxKind::Error`] token has exactly one error spanning it;
    /// other tokens may carry errors over part of their text.
    pub errors: Vec<LexicalError>,
}

/// Lexes the given source text into a flat sequence of tokens, or rejects it as a whole.
pub fn lex(source: &str) -> Result<LexResult, InputRejection> {
    if let Some(offset) = source.find('\0') {
        return Err(InputRejection {
            offset: text_size(offset),
        });
    }
    let mut lexer = Lexer {
        source,
        pos: 0,
        line_start: 0,
        line_has_tokens: false,
        line_indentation: 0,
        last_kind: None,
        pending_block: None,
        indent_stack: vec![0],
        out: LexResult::default(),
    };
    lexer.run();
    Ok(lexer.out)
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    line_start: usize,
    line_has_tokens: bool,
    line_indentation: usize,
    last_kind: Option<SyntaxKind>,
    pending_block: Option<usize>,
    indent_stack: Vec<usize>,
    out: LexResult,
}

const BOM: char = '\u{FEFF}';

impl Lexer<'_> {
    fn run(&mut self) {
        if self.rest().starts_with(BOM) {
            let start = self.pos;
            self.pos += BOM.len_utf8();
            self.push(SyntaxKind::Whitespace, start);
            self.line_start = self.pos;
        }
        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                '\r' => {
                    self.pos += 1;
                    if self.peek() == Some('\n') {
                        self.pos += 1;
                    }
                    self.end_line(start);
                }
                '\n' => {
                    self.pos += 1;
                    self.end_line(start);
                }
                ' ' => {
                    self.bump_while(|c| c == ' ');
                    if self.opens_markdown_body() {
                        self.markdown_body();
                    } else {
                        self.push(SyntaxKind::Whitespace, start);
                    }
                }
                '#' => {
                    self.bump_while(|c| !is_line_terminator(c));
                    self.push(SyntaxKind::Comment, start);
                }
                _ if !self.line_has_tokens => self.begin_line(),
                '\t' => {
                    self.bump_while(|c| c == '\t');
                    self.push_error(LexicalErrorKind::IllegalTab, start);
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
                    self.push(SyntaxKind::At, start);
                }
                c if c.is_ascii_digit() => self.number(start),
                c if c.is_ascii_alphabetic() => self.word(start),
                '"' => self.string(start),
                c => {
                    self.pos += c.len_utf8();
                    self.push_error(LexicalErrorKind::UnexpectedCharacter, start);
                }
            }
        }
        self.end_input();
    }

    fn opens_markdown_body(&self) -> bool {
        let Some(reference) = self.pending_block else {
            return false;
        };
        !self.line_has_tokens
            && self.pos - self.line_start > reference
            && self.peek().is_some_and(|c| !is_line_terminator(c))
    }

    fn markdown_body(&mut self) {
        let reference = self
            .pending_block
            .take()
            .expect("checked by opens_markdown_body");
        let start = self.line_start;
        let mut end = self.line_end(self.pos);
        let mut cursor = end;
        while cursor < self.source.len() {
            let content_end = self.line_content_end(cursor);
            let line = &self.source[cursor..content_end];
            let indentation = line.bytes().take_while(|b| *b == b' ').count();
            let next = self.line_end(content_end);
            if indentation == line.len() {
                cursor = next;
            } else if indentation > reference {
                cursor = next;
                end = next;
            } else {
                break;
            }
        }
        self.pos = end;
        self.push(SyntaxKind::MarkdownBlock, start);
        self.line_start = self.pos;
        self.line_has_tokens = false;
    }

    fn line_content_end(&self, from: usize) -> usize {
        self.source[from..]
            .find(is_line_terminator)
            .map_or(self.source.len(), |i| from + i)
    }

    fn line_end(&self, from: usize) -> usize {
        let content_end = self.line_content_end(from);
        let rest = &self.source[content_end..];
        if rest.starts_with("\r\n") {
            content_end + 2
        } else if rest.starts_with(['\r', '\n']) {
            content_end + 1
        } else {
            content_end
        }
    }

    fn begin_line(&mut self) {
        self.pending_block = None;
        self.line_has_tokens = true;
        let indentation = self.pos - self.line_start;
        self.line_indentation = indentation;
        let current = *self
            .indent_stack
            .last()
            .expect("indent stack is never empty");
        if indentation > current {
            self.indent_stack.push(indentation);
            self.push(SyntaxKind::Indent, self.pos);
            return;
        }
        while *self
            .indent_stack
            .last()
            .expect("indent stack is never empty")
            > indentation
        {
            self.indent_stack.pop();
            self.push(SyntaxKind::Dedent, self.pos);
        }
        if *self
            .indent_stack
            .last()
            .expect("indent stack is never empty")
            != indentation
        {
            self.report(
                LexicalErrorKind::InconsistentDedent,
                self.line_start,
                self.pos,
            );
        }
    }

    fn end_line(&mut self, start: usize) {
        let kind = if self.line_has_tokens {
            SyntaxKind::Newline
        } else {
            SyntaxKind::BlankLine
        };
        self.push(kind, start);
        if self.line_has_tokens {
            let opens = self.last_kind == Some(SyntaxKind::Colon);
            self.pending_block = opens.then_some(self.line_indentation);
        }
        self.line_start = self.pos;
        self.line_has_tokens = false;
        self.last_kind = None;
    }

    fn end_input(&mut self) {
        if self.line_has_tokens {
            self.push(SyntaxKind::Newline, self.pos);
        }
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.push(SyntaxKind::Dedent, self.pos);
        }
    }

    fn number(&mut self, start: usize) {
        self.bump_while(|c| is_word_char(c) || c == '.');
        let text = &self.source[start..self.pos];
        if text.bytes().all(|b| b.is_ascii_digit()) {
            self.push(SyntaxKind::IntegerLiteral, start);
        } else if is_date_shaped(text) {
            self.push(SyntaxKind::DateLiteral, start);
        } else {
            self.push_error(LexicalErrorKind::MalformedNumber, start);
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
                self.push_error(LexicalErrorKind::MalformedUrl, start);
            } else {
                self.push(SyntaxKind::UrlLiteral, start);
            }
            return;
        }
        self.push(SyntaxKind::Word, start);
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
                            self.report(LexicalErrorKind::InvalidEscape, escape_start, self.pos);
                        }
                        _ => {}
                    }
                }
                Some(c) if !is_line_terminator(c) => self.pos += c.len_utf8(),
                _ => {
                    self.push_error(LexicalErrorKind::UnterminatedString, start);
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
        if is_significant(kind) {
            self.last_kind = Some(kind);
        }
    }

    fn report(&mut self, kind: LexicalErrorKind, start: usize, end: usize) {
        let range = TextRange::new(text_size(start), text_size(end));
        self.out.errors.push(LexicalError { kind, range });
    }

    fn push_error(&mut self, kind: LexicalErrorKind, start: usize) {
        self.report(kind, start, self.pos);
        self.push(SyntaxKind::Error, start);
    }
}

fn text_size(offset: usize) -> TextSize {
    TextSize::try_from(offset).expect("source longer than 4 GiB")
}

fn is_significant(kind: SyntaxKind) -> bool {
    !kind.is_trivia()
        && !matches!(
            kind,
            SyntaxKind::Indent
                | SyntaxKind::Dedent
                | SyntaxKind::Newline
                | SyntaxKind::BlankLine
                | SyntaxKind::MarkdownBlock
        )
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

    use super::{InputRejection, LexicalError, LexicalErrorKind, lex, text_size};
    use crate::Keyword;
    use crate::SyntaxKind::{
        self, At, BlankLine, Colon, Comma, Comment, DateLiteral, Dedent, Error, Indent,
        IntegerLiteral, MarkdownBlock, Newline, StringLiteral, UrlLiteral, Whitespace, Word,
    };

    fn check(source: &str, expected: &[(SyntaxKind, &str)]) -> Vec<LexicalError> {
        let lexed = lex(source).expect("accepted");
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
    fn every_line_terminator_form_ends_a_line() {
        check("yes\n", &[(Word, "yes"), (Newline, "\n")]);
        check("yes\r\n", &[(Word, "yes"), (Newline, "\r\n")]);
        check("yes\r", &[(Word, "yes"), (Newline, "\r")]);
    }

    #[test]
    fn terminators_of_blank_lines_are_blank_lines() {
        check("\n", &[(BlankLine, "\n")]);
        check("\r\n", &[(BlankLine, "\r\n")]);
        check(
            "\n\r\n\r",
            &[(BlankLine, "\n"), (BlankLine, "\r\n"), (BlankLine, "\r")],
        );
        check(
            "\n  \n",
            &[(BlankLine, "\n"), (Whitespace, "  "), (BlankLine, "\n")],
        );
        check(
            "yes\n\n",
            &[(Word, "yes"), (Newline, "\n"), (BlankLine, "\n")],
        );
    }

    #[test]
    fn comment_runs_to_end_of_line() {
        check(
            "# a comment\n",
            &[(Comment, "# a comment"), (BlankLine, "\n")],
        );
        check(
            "yes # c\n",
            &[
                (Word, "yes"),
                (Whitespace, " "),
                (Comment, "# c"),
                (Newline, "\n"),
            ],
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
            &[
                (Comment, "# tabs\tand \"quotes\" @ids"),
                (BlankLine, "\r\n"),
            ],
        );
    }

    #[test]
    fn byte_order_mark_is_whitespace() {
        check("\u{FEFF}# x", &[(Whitespace, "\u{FEFF}"), (Comment, "# x")]);
        check("\u{FEFF}", &[(Whitespace, "\u{FEFF}")]);
    }

    #[test]
    fn tab_is_an_error() {
        let errors = check(
            "\t\t# c",
            &[(Error, "\t\t"), (Comment, "# c"), (Newline, "")],
        );
        assert_eq!(
            errors,
            [LexicalError {
                kind: LexicalErrorKind::IllegalTab,
                range: range(0, 2)
            }]
        );
    }

    #[test]
    fn every_keyword_lexes_as_a_word() {
        for keyword in Keyword::ALL.iter().copied() {
            let errors = check(keyword.text(), &[(Word, keyword.text()), (Newline, "")]);
            assert_eq!(errors.len(), 0, "{keyword:?}");
        }
    }

    #[test]
    fn any_word_is_accepted_whatever_its_text() {
        check("Guide", &[(Word, "Guide"), (Newline, "")]);
        check(
            "foo bar",
            &[
                (Word, "foo"),
                (Whitespace, " "),
                (Word, "bar"),
                (Newline, ""),
            ],
        );
        check("no_position", &[(Word, "no_position"), (Newline, "")]);
        check("yes2", &[(Word, "yes2"), (Newline, "")]);
        check("a-b_c9", &[(Word, "a-b_c9"), (Newline, "")]);
    }

    #[test]
    fn a_word_begins_with_a_letter() {
        for (source, rest) in [("-foo", "foo"), ("_x", "x")] {
            let errors = check(
                source,
                &[(Error, &source[..1]), (Word, rest), (Newline, "")],
            );
            assert_eq!(
                errors[0].kind,
                LexicalErrorKind::UnexpectedCharacter,
                "{source:?}"
            );
            assert_eq!(errors[0].range, range(0, 1), "{source:?}");
        }
    }

    #[test]
    fn words_stop_at_comments_and_terminators() {
        check(
            "foo#bar\nbaz",
            &[
                (Word, "foo"),
                (Comment, "#bar"),
                (Newline, "\n"),
                (Word, "baz"),
                (Newline, ""),
            ],
        );
    }

    #[test]
    fn the_at_sign_is_a_punctuator_of_its_own() {
        check("@a", &[(At, "@"), (Word, "a"), (Newline, "")]);
        check("@Abc", &[(At, "@"), (Word, "Abc"), (Newline, "")]);
        check(
            "@a @b",
            &[
                (At, "@"),
                (Word, "a"),
                (Whitespace, " "),
                (At, "@"),
                (Word, "b"),
                (Newline, ""),
            ],
        );
        check(
            "@ a",
            &[(At, "@"), (Whitespace, " "), (Word, "a"), (Newline, "")],
        );
        check("@", &[(At, "@"), (Newline, "")]);
        check("@9", &[(At, "@"), (IntegerLiteral, "9"), (Newline, "")]);
        let errors = check(
            "@-x",
            &[(At, "@"), (Error, "-"), (Word, "x"), (Newline, "")],
        );
        assert_eq!(errors[0].kind, LexicalErrorKind::UnexpectedCharacter);
    }

    #[test]
    fn input_containing_a_null_character_is_rejected_as_a_whole() {
        assert_eq!(
            lex("guide \"G\"\n\0"),
            Err(InputRejection {
                offset: text_size(10)
            })
        );
        assert_eq!(
            lex("\0").unwrap_err().to_string(),
            "the input contains U+0000 at byte 0 and is not an Oval document"
        );
    }

    #[test]
    fn punctuators() {
        check("note:", &[(Word, "note"), (Colon, ":"), (Newline, "")]);
        check(
            "yes,no",
            &[(Word, "yes"), (Comma, ","), (Word, "no"), (Newline, "")],
        );
        check(
            "@a,@b",
            &[
                (At, "@"),
                (Word, "a"),
                (Comma, ","),
                (At, "@"),
                (Word, "b"),
                (Newline, ""),
            ],
        );
        check("::", &[(Colon, ":"), (Colon, ":"), (Newline, "")]);
    }

    #[test]
    fn a_header_line() {
        check(
            "campaign incumbent @bsantos",
            &[
                (Word, "campaign"),
                (Whitespace, " "),
                (Word, "incumbent"),
                (Whitespace, " "),
                (At, "@"),
                (Word, "bsantos"),
                (Newline, ""),
            ],
        );
    }

    #[test]
    fn unexpected_characters_are_one_error_each() {
        let errors = check("!", &[(Error, "!"), (Newline, "")]);
        assert_eq!(errors[0].kind, LexicalErrorKind::UnexpectedCharacter);
        check("!?", &[(Error, "!"), (Error, "?"), (Newline, "")]);
    }

    #[test]
    fn string_literals() {
        check("\"abc\"", &[(StringLiteral, "\"abc\""), (Newline, "")]);
        check("\"\"", &[(StringLiteral, "\"\""), (Newline, "")]);
        check(
            "\"José Cisneros\"",
            &[(StringLiteral, "\"José Cisneros\""), (Newline, "")],
        );
    }

    #[test]
    fn strings_may_contain_anything_but_quotes_and_terminators() {
        check(
            "\"# not a comment\"",
            &[(StringLiteral, "\"# not a comment\""), (Newline, "")],
        );
        check(
            "\"@not-an-id, yes:\"",
            &[(StringLiteral, "\"@not-an-id, yes:\""), (Newline, "")],
        );
        check("\"a\tb\"", &[(StringLiteral, "\"a\tb\""), (Newline, "")]);
    }

    #[test]
    fn string_escapes() {
        let errors = check(
            "\"a\\\"b\"",
            &[(StringLiteral, "\"a\\\"b\""), (Newline, "")],
        );
        assert_eq!(errors.len(), 0, "{errors:?}");
        check(
            "\"a\\\\b\"",
            &[(StringLiteral, "\"a\\\\b\""), (Newline, "")],
        );
        check(
            "\"\\\"\\\\\"",
            &[(StringLiteral, "\"\\\"\\\\\""), (Newline, "")],
        );
    }

    #[test]
    fn invalid_escapes_keep_the_string_and_report_the_escape() {
        let errors = check("\"a\\nb\"", &[(StringLiteral, "\"a\\nb\""), (Newline, "")]);
        assert_eq!(
            errors,
            [LexicalError {
                kind: LexicalErrorKind::InvalidEscape,
                range: range(2, 4)
            }]
        );
        let errors = check(
            "\"\\x\\y\"",
            &[(StringLiteral, "\"\\x\\y\""), (Newline, "")],
        );
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert_eq!(errors[1].range, range(3, 5));
    }

    #[test]
    fn unterminated_strings_stop_at_the_line_end() {
        let errors = check("\"abc", &[(Error, "\"abc"), (Newline, "")]);
        assert_eq!(
            errors,
            [LexicalError {
                kind: LexicalErrorKind::UnterminatedString,
                range: range(0, 4)
            }]
        );
        check(
            "\"abc\n\"ok\"",
            &[
                (Error, "\"abc"),
                (Newline, "\n"),
                (StringLiteral, "\"ok\""),
                (Newline, ""),
            ],
        );
        check("\"abc\r\n", &[(Error, "\"abc"), (Newline, "\r\n")]);
        check("\"", &[(Error, "\""), (Newline, "")]);
    }

    #[test]
    fn trailing_backslash_is_only_unterminated() {
        let errors = check("\"abc\\", &[(Error, "\"abc\\"), (Newline, "")]);
        assert_eq!(
            errors,
            [LexicalError {
                kind: LexicalErrorKind::UnterminatedString,
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
                (Word, "supporters"),
                (Whitespace, " "),
                (StringLiteral, "\"a\""),
                (Comma, ","),
                (Whitespace, " "),
                (StringLiteral, "\"b\""),
                (Newline, ""),
            ],
        );
        check(
            "designation\"x\"# c",
            &[
                (Word, "designation"),
                (StringLiteral, "\"x\""),
                (Comment, "# c"),
                (Newline, ""),
            ],
        );
    }

    #[test]
    fn multibyte_characters_are_measured_in_bytes() {
        let errors = check("é ", &[(Error, "é"), (Whitespace, " "), (Newline, "")]);
        assert_eq!(errors[0].range, range(0, 2));
    }

    #[test]
    fn integer_literals() {
        check("3", &[(IntegerLiteral, "3"), (Newline, "")]);
        check("007", &[(IntegerLiteral, "007"), (Newline, "")]);
        check(
            "3,4",
            &[
                (IntegerLiteral, "3"),
                (Comma, ","),
                (IntegerLiteral, "4"),
                (Newline, ""),
            ],
        );
        check(
            "3#c",
            &[(IntegerLiteral, "3"), (Comment, "#c"), (Newline, "")],
        );
        check(
            "ranked max 10",
            &[
                (Word, "ranked"),
                (Whitespace, " "),
                (Word, "max"),
                (Whitespace, " "),
                (IntegerLiteral, "10"),
                (Newline, ""),
            ],
        );
    }

    #[test]
    fn date_literals_match_the_shape_only() {
        check("2026-11-05", &[(DateLiteral, "2026-11-05"), (Newline, "")]);
        check("0000-00-00", &[(DateLiteral, "0000-00-00"), (Newline, "")]);
        check("2026-13-45", &[(DateLiteral, "2026-13-45"), (Newline, "")]);
        check(
            "date 2026-11-05\n",
            &[
                (Word, "date"),
                (Whitespace, " "),
                (DateLiteral, "2026-11-05"),
                (Newline, "\n"),
            ],
        );
    }

    #[test]
    fn the_version_marker_is_a_keyword_and_an_integer() {
        check(
            "oval 1\n",
            &[
                (Word, "oval"),
                (Whitespace, " "),
                (IntegerLiteral, "1"),
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
            "0.1",
            "1.0",
            "12.30",
            "01.1",
            "1.01",
            "1.",
            "1.x",
            "1.2.3",
            "0.1-rc",
        ] {
            let errors = check(source, &[(Error, source), (Newline, "")]);
            assert_eq!(
                errors[0].kind,
                LexicalErrorKind::MalformedNumber,
                "{source:?}"
            );
        }
    }

    #[test]
    fn url_literals_run_to_whitespace() {
        check(
            "https://leepatel2026.example",
            &[(UrlLiteral, "https://leepatel2026.example"), (Newline, "")],
        );
        check(
            "http://a.b/c?d=1&e=2#frag",
            &[(UrlLiteral, "http://a.b/c?d=1&e=2#frag"), (Newline, "")],
        );
        check(
            "https://x.y/\"quoted\"",
            &[(UrlLiteral, "https://x.y/\"quoted\""), (Newline, "")],
        );
        check(
            "website https://x.y # c\n",
            &[
                (Word, "website"),
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
        let errors = check("https://", &[(Error, "https://"), (Newline, "")]);
        assert_eq!(errors[0].kind, LexicalErrorKind::MalformedUrl);
        let errors = check(
            "https:// x",
            &[
                (Error, "https://"),
                (Whitespace, " "),
                (Word, "x"),
                (Newline, ""),
            ],
        );
        assert_eq!(errors[0].kind, LexicalErrorKind::MalformedUrl);
        check("https:", &[(Word, "https"), (Colon, ":"), (Newline, "")]);
        let errors = check(
            "ftp://x",
            &[
                (Word, "ftp"),
                (Colon, ":"),
                (Error, "/"),
                (Error, "/"),
                (Word, "x"),
                (Newline, ""),
            ],
        );
        assert_eq!(errors[0].kind, LexicalErrorKind::UnexpectedCharacter);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn indent_and_dedent_are_zero_length_tokens_at_the_first_token() {
        check(
            "guide\n  note\nparty",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Newline, "\n"),
                (Dedent, ""),
                (Word, "party"),
                (Newline, ""),
            ],
        );
    }

    #[test]
    fn open_blocks_are_closed_at_end_of_input() {
        check(
            "guide\n  note",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Newline, ""),
                (Dedent, ""),
            ],
        );
        check(
            "guide\n  note\n    yes\n",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Newline, "\n"),
                (Whitespace, "    "),
                (Indent, ""),
                (Word, "yes"),
                (Newline, "\n"),
                (Dedent, ""),
                (Dedent, ""),
            ],
        );
    }

    #[test]
    fn one_line_may_close_several_levels() {
        check(
            "guide\n  note\n    yes\nparty",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Newline, "\n"),
                (Whitespace, "    "),
                (Indent, ""),
                (Word, "yes"),
                (Newline, "\n"),
                (Dedent, ""),
                (Dedent, ""),
                (Word, "party"),
                (Newline, ""),
            ],
        );
    }

    #[test]
    fn indentation_width_is_free() {
        check(
            "guide\n   note\n          yes\n   no",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (Whitespace, "   "),
                (Indent, ""),
                (Word, "note"),
                (Newline, "\n"),
                (Whitespace, "          "),
                (Indent, ""),
                (Word, "yes"),
                (Newline, "\n"),
                (Whitespace, "   "),
                (Dedent, ""),
                (Word, "no"),
                (Newline, ""),
                (Dedent, ""),
            ],
        );
    }

    #[test]
    fn blank_and_comment_lines_do_not_affect_layout() {
        check(
            "guide\n\n      # deep comment\n  \n  note\n",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (BlankLine, "\n"),
                (Whitespace, "      "),
                (Comment, "# deep comment"),
                (BlankLine, "\n"),
                (Whitespace, "  "),
                (BlankLine, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
        );
    }

    #[test]
    fn inconsistent_dedent_is_reported_on_the_leading_whitespace() {
        let errors = check(
            "guide\n    note\n  party",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (Whitespace, "    "),
                (Indent, ""),
                (Word, "note"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Dedent, ""),
                (Word, "party"),
                (Newline, ""),
            ],
        );
        assert_eq!(
            errors,
            [LexicalError {
                kind: LexicalErrorKind::InconsistentDedent,
                range: range(15, 17)
            }]
        );
    }

    #[test]
    fn an_indented_first_line_opens_a_block() {
        check(
            "  guide",
            &[
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "guide"),
                (Newline, ""),
                (Dedent, ""),
            ],
        );
        check(
            "\u{FEFF}  guide",
            &[
                (Whitespace, "\u{FEFF}"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "guide"),
                (Newline, ""),
                (Dedent, ""),
            ],
        );
    }

    #[test]
    fn a_leading_tab_is_content_at_indentation_zero() {
        let errors = check(
            "\tguide\n",
            &[(Error, "\t"), (Word, "guide"), (Newline, "\n")],
        );
        assert_eq!(errors[0].kind, LexicalErrorKind::IllegalTab);
    }

    fn opener(keyword: Keyword) -> [(SyntaxKind, &'static str); 3] {
        [(Word, keyword.text()), (Colon, ":"), (Newline, "\n")]
    }

    #[test]
    fn markdown_body_is_one_raw_token() {
        let [k, c, n] = opener(Keyword::Note);
        check(
            "note:\n  Hello, world.\n",
            &[k, c, n, (MarkdownBlock, "  Hello, world.\n")],
        );
    }

    #[test]
    fn markdown_body_keeps_internal_blank_lines_and_indentation() {
        let [k, c, n] = opener(Keyword::Justification);
        check(
            "justification:\n  First.\n\n   \n  Second.\n",
            &[k, c, n, (MarkdownBlock, "  First.\n\n   \n  Second.\n")],
        );
    }

    #[test]
    fn markdown_body_is_opaque() {
        let [k, c, n] = opener(Keyword::Note);
        let body = "  # heading @id \"quote\" yes:\ttab\n";
        let errors = check(&format!("note:\n{body}"), &[k, c, n, (MarkdownBlock, body)]);
        assert_eq!(errors.len(), 0, "{errors:?}");
    }

    #[test]
    fn markdown_body_ends_at_a_line_not_indented_past_the_keyword() {
        check(
            "guide\n  note:\n    body\n  party\n",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "    body\n"),
                (Whitespace, "  "),
                (Word, "party"),
                (Newline, "\n"),
                (Dedent, ""),
            ],
        );
    }

    #[test]
    fn markdown_body_lines_may_vary_in_depth() {
        check(
            "guide\n  note:\n      deep\n    less\n  party",
            &[
                (Word, "guide"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "note"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "      deep\n    less\n"),
                (Whitespace, "  "),
                (Word, "party"),
                (Newline, ""),
                (Dedent, ""),
            ],
        );
    }

    #[test]
    fn markdown_body_ends_at_end_of_input() {
        let [k, c, n] = opener(Keyword::Note);
        check("note:\n  body", &[k, c, n, (MarkdownBlock, "  body")]);
        check(
            "note:\n  body\r\n",
            &[k, c, n, (MarkdownBlock, "  body\r\n")],
        );
    }

    #[test]
    fn blank_lines_around_a_body_are_not_part_of_it() {
        let [k, c, n] = opener(Keyword::Note);
        check(
            "note:\n\n  body\n\n\nparty",
            &[
                k,
                c,
                n,
                (BlankLine, "\n"),
                (MarkdownBlock, "  body\n"),
                (BlankLine, "\n"),
                (BlankLine, "\n"),
                (Word, "party"),
                (Newline, ""),
            ],
        );
    }

    #[test]
    fn empty_body_produces_no_token() {
        let [k, c, n] = opener(Keyword::Note);
        check("note:\nparty", &[k, c, n, (Word, "party"), (Newline, "")]);
        check(
            "note:\n\nparty",
            &[k, c, n, (BlankLine, "\n"), (Word, "party"), (Newline, "")],
        );
        check("note:\n", &[k, c, n]);
        check("note:", &[(Word, "note"), (Colon, ":"), (Newline, "")]);
    }

    #[test]
    fn a_lower_indented_comment_ends_a_body() {
        let [k, c, n] = opener(Keyword::Note);
        check(
            "note:\n  body\n# c\n",
            &[
                k,
                c,
                n,
                (MarkdownBlock, "  body\n"),
                (Comment, "# c"),
                (BlankLine, "\n"),
            ],
        );
    }

    #[test]
    fn content_after_the_colon_does_not_open_a_body() {
        check(
            "note: yes\n  no",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Whitespace, " "),
                (Word, "yes"),
                (Newline, "\n"),
                (Whitespace, "  "),
                (Indent, ""),
                (Word, "no"),
                (Newline, ""),
                (Dedent, ""),
            ],
        );
    }

    #[test]
    fn any_line_ending_in_a_colon_opens_a_body() {
        check(
            "@a:\n  yes",
            &[
                (At, "@"),
                (Word, "a"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  yes"),
            ],
        );
        check(
            "recommend yes:\n  text\n",
            &[
                (Word, "recommend"),
                (Whitespace, " "),
                (Word, "yes"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  text\n"),
            ],
        );
        check(
            "nte:\n  text\n",
            &[
                (Word, "nte"),
                (Colon, ":"),
                (Newline, "\n"),
                (MarkdownBlock, "  text\n"),
            ],
        );
    }

    #[test]
    fn a_trailing_comment_on_the_opener_line_is_allowed() {
        check(
            "note: # c\n  body\n",
            &[
                (Word, "note"),
                (Colon, ":"),
                (Whitespace, " "),
                (Comment, "# c"),
                (Newline, "\n"),
                (MarkdownBlock, "  body\n"),
            ],
        );
    }
}
