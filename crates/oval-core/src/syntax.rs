use oval_syntax::SyntaxKind;
use oval_syntax::lexer::{LexicalError, LexicalErrorKind};
use oval_syntax::parser::{ParseError, ParseErrorKind, SyntaxError};

use crate::{Diagnostic, DiagnosticCode};

pub(crate) fn diagnostic(source: &str, error: &SyntaxError) -> Diagnostic {
    let message = match error {
        SyntaxError::Lexical(error) => lexical_message(error),
        SyntaxError::Parse(error) => parse_message(source, error),
    };
    let code = match error {
        SyntaxError::Lexical(error) => DiagnosticCode::Lexical(error.kind),
        SyntaxError::Parse(error) => DiagnosticCode::Syntax(error.kind),
    };
    Diagnostic::new(code, error.range(), message)
}

fn lexical_message(error: &LexicalError) -> String {
    match error.kind {
        LexicalErrorKind::IllegalTab => {
            "tabs are not allowed outside strings, comments, and Markdown bodies"
        }
        LexicalErrorKind::UnexpectedCharacter => "unexpected character",
        LexicalErrorKind::UnterminatedString => "this string has no closing quote",
        LexicalErrorKind::InvalidEscape => {
            "only `\\\"` and `\\\\` may follow a backslash in a string"
        }
        LexicalErrorKind::MalformedNumber => "this is neither a number nor a date",
        LexicalErrorKind::MalformedUrl => "a URL needs something after `://`",
        LexicalErrorKind::InconsistentDedent => {
            "this line is indented to a level that no enclosing block uses"
        }
    }
    .to_owned()
}

fn parse_message(source: &str, error: &ParseError) -> String {
    let found = match error.found {
        Some(SyntaxKind::Word) => format!("`{}`", leading_word(&source[error.range])),
        Some(kind) => describe(kind),
        None => "the end of the file".to_owned(),
    };
    let expected: Vec<String> = error.expected.iter().map(|k| describe(*k)).collect();
    match error.kind {
        ParseErrorKind::UnexpectedToken | ParseErrorKind::UnexpectedEntry => {
            if expected.is_empty() {
                format!("unexpected {found}")
            } else {
                format!("expected {}, found {found}", list(&expected))
            }
        }
        ParseErrorKind::MissingIdentifier => "an identifier is required here: `@` and a name".to_owned(),
        ParseErrorKind::MissingName => "a declaration needs a quoted name after its identifier".to_owned(),
        ParseErrorKind::MissingContestKind => {
            "a contest needs a kind after its name: `single`, `multiple`, `ranked`, or `retention`"
                .to_owned()
        }
        ParseErrorKind::MissingValue => format!("expected {}, found {found}", list(&expected)),
        ParseErrorKind::MissingBody => {
            "this Markdown field has no body; indent the text under it or omit the field".to_owned()
        }
        ParseErrorKind::MissingBlock => "expected an indented block after this line".to_owned(),
        ParseErrorKind::UnexpectedBlock => "this line does not take an indented block".to_owned(),
        ParseErrorKind::RepeatedField => format!("{found} appears more than once in this block"),
        ParseErrorKind::RepeatedFlag => format!("{found} appears more than once"),
        ParseErrorKind::MisplacedEntry => {
            format!("{found} cannot appear here; expected {}", list(&expected))
        }
        ParseErrorKind::EmptyRecommendation => {
            "a `recommend` block needs a `support` or `oppose` line; write `recommend no-position` to take no position"
                .to_owned()
        }
    }
}

fn leading_word(text: &str) -> &str {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_')))
        .unwrap_or(text.len());
    &text[..end]
}

fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} or {b}"),
        [init @ .., last] => format!("{}, or {last}", init.join(", ")),
    }
}

fn describe(kind: SyntaxKind) -> String {
    if let Some(keyword) = kind.as_keyword() {
        return format!("`{}`", keyword.text());
    }
    match kind {
        SyntaxKind::At => "an identifier",
        SyntaxKind::Word => "a word",
        SyntaxKind::StringLiteral => "a string",
        SyntaxKind::IntegerLiteral => "a number",
        SyntaxKind::DateLiteral => "a date",
        SyntaxKind::UrlLiteral => "a URL",
        SyntaxKind::Colon => "`:`",
        SyntaxKind::Comma => "`,`",
        SyntaxKind::MarkdownBlock => "a Markdown body",
        SyntaxKind::Newline => "the end of the line",
        SyntaxKind::Indent => "an indented block",
        SyntaxKind::Dedent => "the end of the block",
        SyntaxKind::Error => "unrecognized text",
        _ => "something else",
    }
    .to_owned()
}
