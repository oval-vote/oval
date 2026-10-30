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
        Self {
            range,
            severity: Severity::Error,
            code,
            message: message.into(),
        }
    }
    pub fn warning(range: TextRange, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            range,
            severity: Severity::Warning,
            code,
            message: message.into(),
        }
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
        let line = self
            .line_starts
            .partition_point(|&s| s <= offset)
            .saturating_sub(1);
        let start = self.line_starts[line];
        let col = text[usize::from(start)..usize::from(offset)]
            .chars()
            .count();
        (line + 1, col + 1)
    }
}
