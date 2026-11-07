//! The `oval` command line tool.
//!
//! `oval` reads Oval voting guides from files and checks, compiles, or formats them with
//! [`oval_core`] and [`oval_format`]. It has three subcommands.
//!
//! - `oval check FILE...` parses and validates each file and prints every diagnostic, errors and
//!   warnings alike, to standard output. With `--strict`, a warning fails the file as an error
//!   does.
//! - `oval compile FILE` compiles one file to its canonical JSON form, Section 5 of the
//!   specification, on standard output, either compact or indented with `--pretty`. Diagnostics
//!   go to standard error.
//! - `oval fmt FILE...` rewrites each file in the canonical style of Section 6. With `--check` it
//!   rewrites nothing and instead names each file that would change. A file with lexical or
//!   syntax errors is left as it is and its diagnostics go to standard error.
//!
//! Diagnostics are printed one per line as `file:line:column: severity[identifier]: message`.
//! `line` and `column` are one-based and `column` counts characters; `severity` is `error` or
//! `warning`; `identifier` is the diagnostic's identifier in the specification, such as
//! `missing-guide-field`. With `--json` each subcommand prints one JSON array instead, with an
//! object per diagnostic whose fields are `path`, `code`, `severity`, `message`, `start` and `end`
//! as `{ line, column }`, and `range` as `{ start, end }` in byte offsets. `check` prints the
//! array even when it is empty; `compile` and `fmt` print it only when there is something to
//! report.
//!
//! The exit status is 0 on success, including when the only diagnostics are warnings and `--strict`
//! is not given. It is 1 when a checked file has an error, or a warning under `--strict`, when a
//! compiled file is invalid, or when a formatted file has syntax errors or, under `--check`, would
//! change. It is 2 when a file cannot be read or written, contains
//! U+0000, or its compiled form cannot be written as JSON.

mod line_index;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oval_core::{CompileError, Report, Severity};
use oval_format::FormatError;
use serde::Serialize;

use crate::line_index::LineIndex;

/// The command line, as clap parses it.
#[derive(Parser)]
#[command(name = "oval", version, about = "Tools for Oval voting guides")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// The subcommands.
#[derive(Subcommand)]
enum Command {
    /// Parse and validate one or more documents.
    Check {
        /// Files to check.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Print diagnostics as a JSON array instead of text lines.
        #[arg(long)]
        json: bool,
        /// Fail when a file has warnings, not only errors.
        #[arg(long)]
        strict: bool,
    },
    /// Compile a document to its canonical JSON form.
    Compile {
        /// The file to compile.
        file: PathBuf,
        /// Indent the output for reading.
        #[arg(long)]
        pretty: bool,
        /// Print diagnostics as a JSON array instead of text lines.
        #[arg(long)]
        json: bool,
    },
    /// Rewrite one or more documents in the canonical style.
    Fmt {
        /// Files to format.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Report files that would change instead of rewriting them.
        #[arg(long)]
        check: bool,
        /// Print diagnostics as a JSON array instead of text lines.
        #[arg(long)]
        json: bool,
    },
}

/// A one-based line and column, as `--json` output reports positions.
#[derive(Serialize)]
struct Position {
    line: usize,
    column: usize,
}

/// A range of byte offsets, as `--json` output reports it.
#[derive(Serialize)]
struct ByteRange {
    start: u32,
    end: u32,
}

/// One diagnostic as `--json` output prints it.
#[derive(Serialize)]
struct Entry {
    path: String,
    code: &'static str,
    severity: Severity,
    message: String,
    start: Position,
    end: Position,
    range: ByteRange,
}

/// Collects diagnostics and prints them as text lines or, with `--json`, as one array.
struct Printer {
    json: bool,
    entries: Vec<Entry>,
}

impl Printer {
    fn new(json: bool) -> Self {
        Self {
            json,
            entries: Vec::new(),
        }
    }

    fn report(
        &mut self,
        out: &mut impl Write,
        path: &std::path::Path,
        source: &str,
        report: &Report,
    ) {
        let index = LineIndex::new(source);
        for diagnostic in report.diagnostics() {
            let (line, column) = index.line_column(diagnostic.range.start());
            if self.json {
                let (end_line, end_column) = index.line_column(diagnostic.range.end());
                self.entries.push(Entry {
                    path: path.display().to_string(),
                    code: diagnostic.code.identifier(),
                    severity: diagnostic.severity,
                    message: diagnostic.message.clone(),
                    start: Position { line, column },
                    end: Position {
                        line: end_line,
                        column: end_column,
                    },
                    range: ByteRange {
                        start: diagnostic.range.start().into(),
                        end: diagnostic.range.end().into(),
                    },
                });
            } else {
                let severity = match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                };
                let _ = writeln!(
                    out,
                    "{}:{line}:{column}: {severity}[{}]: {}",
                    path.display(),
                    diagnostic.code,
                    diagnostic.message
                );
            }
        }
    }

    fn finish(self, out: &mut impl Write, even_when_empty: bool) {
        if self.json && (even_when_empty || !self.entries.is_empty()) {
            let json = serde_json::to_string(&self.entries).unwrap_or_else(|_| "[]".to_owned());
            let _ = writeln!(out, "{json}");
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            files,
            json,
            strict,
        } => check(&files, json, strict),
        Command::Compile { file, pretty, json } => compile(&file, pretty, json),
        Command::Fmt { files, check, json } => fmt(&files, check, json),
    }
}

fn fmt(files: &[PathBuf], check_only: bool, json: bool) -> ExitCode {
    let mut worst = ExitCode::SUCCESS;
    let mut printer = Printer::new(json);
    for path in files {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("oval: cannot read {}: {error}", path.display());
                return ExitCode::from(2);
            }
        };
        match oval_format::format(&source) {
            Ok(formatted) if formatted == source => {}
            Ok(_) if check_only => {
                println!("{} would be reformatted", path.display());
                worst = ExitCode::FAILURE;
            }
            Ok(formatted) => {
                if let Err(error) = std::fs::write(path, formatted) {
                    eprintln!("oval: cannot write {}: {error}", path.display());
                    return ExitCode::from(2);
                }
            }
            Err(FormatError::Rejected(rejection)) => {
                eprintln!("oval: {}: {rejection}", path.display());
                return ExitCode::from(2);
            }
            Err(FormatError::SyntaxErrors(_)) => {
                if let Ok(report) = oval_core::check(&source) {
                    printer.report(&mut std::io::stderr(), path, &source, &report);
                }
                worst = ExitCode::FAILURE;
            }
        }
    }
    printer.finish(&mut std::io::stderr(), false);
    worst
}

fn compile(path: &PathBuf, pretty: bool, json: bool) -> ExitCode {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("oval: cannot read {}: {error}", path.display());
            return ExitCode::from(2);
        }
    };
    match oval_core::compile(&source) {
        Ok(compiled) => {
            let json = if pretty {
                compiled.to_json_pretty()
            } else {
                compiled.to_json()
            };
            match json {
                Ok(json) => {
                    println!("{json}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("oval: cannot serialize {}: {error}", path.display());
                    ExitCode::from(2)
                }
            }
        }
        Err(CompileError::Rejected(rejection)) => {
            eprintln!("oval: {}: {rejection}", path.display());
            ExitCode::from(2)
        }
        Err(CompileError::Invalid(report)) => {
            let mut printer = Printer::new(json);
            printer.report(&mut std::io::stderr(), path, &source, &report);
            printer.finish(&mut std::io::stderr(), true);
            ExitCode::FAILURE
        }
    }
}

fn check(files: &[PathBuf], json: bool, strict: bool) -> ExitCode {
    let mut worst = ExitCode::SUCCESS;
    let mut printer = Printer::new(json);
    for path in files {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("oval: cannot read {}: {error}", path.display());
                return ExitCode::from(2);
            }
        };
        let report = match oval_core::check(&source) {
            Ok(report) => report,
            Err(rejection) => {
                eprintln!("oval: {}: {rejection}", path.display());
                return ExitCode::from(2);
            }
        };
        printer.report(&mut std::io::stdout(), path, &source, &report);
        if report.has_errors() || (strict && report.has_diagnostics()) {
            worst = ExitCode::FAILURE;
        }
    }
    printer.finish(&mut std::io::stdout(), true);
    worst
}
