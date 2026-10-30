//! Diagnostics and line/column mapping.

use rowan::{TextRange, TextSize};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

/// A problem found in a file. `code` is a stable kebab-case identifier that
/// tools key on; `message` is for people.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    #[serde(skip)]
    pub range: TextRange,
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

impl Diagnostic {
    pub fn error(range: TextRange, code: &'static str, message: impl Into<String>) -> Self {
        Self { range, severity: Severity::Error, code, message: message.into() }
    }
    pub fn warning(range: TextRange, code: &'static str, message: impl Into<String>) -> Self {
        Self { range, severity: Severity::Warning, code, message: message.into() }
    }
}

/// Whether any diagnostic is an error.
pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

/// The sink the parser, the model, and the linter report into.
#[derive(Clone, Debug, Default)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn error(&mut self, range: TextRange, code: &'static str, message: impl Into<String>) {
        self.0.push(Diagnostic::error(range, code, message));
    }
    pub fn warning(&mut self, range: TextRange, code: &'static str, message: impl Into<String>) {
        self.0.push(Diagnostic::warning(range, code, message));
    }
    pub fn extend(&mut self, more: impl IntoIterator<Item = Diagnostic>) {
        self.0.extend(more);
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Order by position. Stable, so diagnostics at one position keep the
    /// order they were reported in.
    pub fn sort(&mut self) {
        self.0.sort_by_key(|d| (d.range.start(), d.range.end()));
    }
    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.0
    }
}

impl From<Vec<Diagnostic>> for Diagnostics {
    fn from(v: Vec<Diagnostic>) -> Self {
        Self(v)
    }
}

/// Maps byte offsets to 1-based line and column numbers.
pub struct LineIndex {
    line_starts: Vec<TextSize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![TextSize::from(0)];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(TextSize::from(i as u32 + 1));
            }
        }
        Self { line_starts }
    }

    /// 1-based (line, column). Column counts characters, not bytes.
    pub fn line_col(&self, text: &str, offset: TextSize) -> (usize, usize) {
        let line = self.line_starts.partition_point(|&s| s <= offset).saturating_sub(1);
        let start = self.line_starts[line];
        let col = text[usize::from(start)..usize::from(offset)].chars().count();
        (line + 1, col + 1)
    }
}
