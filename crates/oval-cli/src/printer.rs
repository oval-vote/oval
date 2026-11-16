use std::io::{self, Write};
use std::path::Path;

use oval_core::{Diagnostic, Severity};
use serde::Serialize;

use crate::line_index::LineIndex;

pub(crate) struct Printer {
    json: bool,
    entries: Vec<Entry>,
}

#[derive(Serialize)]
struct Entry {
    path: String,
    code: &'static str,
    severity: &'static str,
    message: String,
    start: Position,
    end: Position,
    range: Bytes,
}

#[derive(Serialize)]
struct Position {
    line: usize,
    column: usize,
}

#[derive(Serialize)]
struct Bytes {
    start: usize,
    end: usize,
}

impl Printer {
    pub(crate) fn new(json: bool) -> Self {
        Self {
            json,
            entries: Vec::new(),
        }
    }

    pub(crate) fn print(
        &mut self,
        output: &mut impl Write,
        path: &Path,
        text: &str,
        diagnostics: &[Diagnostic],
    ) -> io::Result<()> {
        let index = LineIndex::new(text);
        for diagnostic in diagnostics {
            let start = usize::from(diagnostic.range.start());
            let end = usize::from(diagnostic.range.end());
            let (line, column) = index.position(start);
            let severity = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            if self.json {
                let (end_line, end_column) = index.position(end);
                self.entries.push(Entry {
                    path: path.display().to_string(),
                    code: diagnostic.code.spec_id(),
                    severity,
                    message: diagnostic.message.clone(),
                    start: Position { line, column },
                    end: Position {
                        line: end_line,
                        column: end_column,
                    },
                    range: Bytes { start, end },
                });
            } else {
                writeln!(
                    output,
                    "{}:{line}:{column}: {severity}[{}]: {}",
                    path.display(),
                    diagnostic.code.spec_id(),
                    diagnostic.message
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn finish(self, output: &mut impl Write, even_when_empty: bool) -> io::Result<()> {
        if self.json && (even_when_empty || !self.entries.is_empty()) {
            serde_json::to_writer(&mut *output, &self.entries)?;
            writeln!(output)?;
        }
        Ok(())
    }
}
