use oval_syntax::SyntaxKind;
use oval_syntax::lexer::{LexicalError, LexicalErrorKind};
use oval_syntax::parser::{ParseError, SyntaxError};

use crate::{Diagnostic, DiagnosticCode, Severity};

pub(crate) fn diagnostic(error: &SyntaxError) -> Diagnostic {
    let message = match error {
        SyntaxError::Lexical(error) => lexical_message(error),
        SyntaxError::Parse(error) => parse_message(error),
    };
    Diagnostic {
        code: DiagnosticCode::Syntax,
        severity: Severity::Error,
        range: error.range(),
        message,
    }
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
