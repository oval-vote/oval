use oval_syntax::SyntaxKind;
use oval_syntax::lexer::{LexError, LexErrorReason};
use oval_syntax::parser::{ParseError, SyntaxError};

use crate::{Diagnostic, DiagnosticCode, Severity};

pub(crate) fn diagnostic(source: &str, error: &SyntaxError) -> Diagnostic {
    let message = match error {
        SyntaxError::Lex(error) => lex_message(source, error),
        SyntaxError::Parse(error) => parse_message(error),
    };
    Diagnostic {
        code: DiagnosticCode::Syntax,
        severity: Severity::Error,
        range: error.range(),
        message,
    }
}

fn lex_message(source: &str, error: &LexError) -> String {
    let text = &source[error.range];
    match error.reason {
        LexErrorReason::IllegalTab => "tabs are not allowed outside strings and Markdown bodies",
        LexErrorReason::MalformedIdentifier => "an identifier is `@` followed by a letter",
        LexErrorReason::UnknownWord => return format!("`{text}` is not a keyword"),
        LexErrorReason::UnexpectedCharacter => "unexpected character",
        LexErrorReason::UnterminatedString => "this string has no closing quote",
        LexErrorReason::InvalidEscape => {
            "only `\\\"` and `\\\\` may follow a backslash in a string"
        }
        LexErrorReason::MalformedNumber => "this is neither a number nor a date",
        LexErrorReason::MalformedUrl => "a URL needs something after `://`",
        LexErrorReason::InconsistentDedent => {
            "this line is indented to a level that no enclosing block uses"
        }
        _ => "unrecognized text",
    }
    .to_owned()
}

fn parse_message(error: &ParseError) -> String {
    let found = match error.found {
        Some(kind) => describe(kind),
        None => "the end of the file".to_owned(),
    };
    if error.expected.is_empty() {
        return format!("unexpected {found}");
    }
    let expected: Vec<String> = error.expected.iter().map(|k| describe(*k)).collect();
    format!("expected {}, found {found}", list(&expected))
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
        SyntaxKind::Identifier => "an identifier",
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
