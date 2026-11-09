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
//! as `{ line, column }`, `range` as `{ start, end }` in byte offsets, and `url`, the address of
//! the diagnostic's entry in the specification. `check` prints the array even when it is empty;
//! `compile` and `fmt` print it only when there is something to report.
//!
//! A file that cannot be processed, because it cannot be read or written, is not UTF-8, or
//! contains U+0000, is reported on standard error as one line starting `oval:`, never in the JSON
//! array, and the remaining files are still processed. Invalid UTF-8 is reported with the line and
//! column of its first bad byte.
//!
//! The exit status is the most severe outcome among the files. It is 0 on success, including when
//! the only diagnostics are warnings and `--strict` is not given. It is 1 when a checked file has
//! an error, or a warning under `--strict`, when a compiled file is invalid, or when a formatted
//! file has syntax errors or, under `--check`, would change. It is 2 when a file cannot be
//! processed, when its compiled form cannot be written as JSON, when output cannot be written, and
//! when the command line itself is invalid. When standard output is closed early, as by `head`,
//! the run stops and the status is 0.

mod line_index;

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oval_core::{CompileError, Diagnostic, InputRejection, Report, Severity};
use oval_format::FormatError;
use oval_syntax::TextSize;
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

/// The outcome of a run or of one file, ordered from best to worst.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ExitStatus {
    /// Nothing failed.
    Success,
    /// A file has errors, has warnings under `--strict`, or would change under `fmt --check`.
    Failure,
    /// A file could not be processed, or output could not be written.
    Error,
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        match status {
            ExitStatus::Success => ExitCode::SUCCESS,
            ExitStatus::Failure => ExitCode::FAILURE,
            ExitStatus::Error => ExitCode::from(2),
        }
    }
}

/// The form diagnostics are printed in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    /// One line per diagnostic.
    Text,
    /// One JSON array for the whole run.
    Json,
}

impl OutputFormat {
    fn from_json_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Text }
    }
}

/// Whether `--json` output prints an array when there is nothing to report.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WhenEmpty {
    /// Print `[]`.
    PrintArray,
    /// Print nothing.
    PrintNothing,
}

/// One of the two output streams.
#[derive(Clone, Copy)]
enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// A reason one file could not be processed.
enum FileError {
    /// The file could not be read.
    Read(io::Error),
    /// The file is not UTF-8; the position is that of its first invalid byte.
    InvalidUtf8 { line: usize, column: usize },
    /// The file contains U+0000.
    Rejected(InputRejection),
    /// The formatted file could not be written back.
    Write(io::Error),
    /// The compiled form could not be written as JSON.
    Serialize(serde_json::Error),
}

/// The run's locked output streams. Standard output is buffered and standard error is not.
struct Output {
    stdout: BufWriter<io::StdoutLock<'static>>,
    stderr: io::StderrLock<'static>,
}

impl Output {
    fn new() -> Self {
        Self {
            stdout: BufWriter::new(io::stdout().lock()),
            stderr: io::stderr().lock(),
        }
    }

    fn stream(&mut self, stream: Stream) -> &mut dyn Write {
        match stream {
            Stream::Stdout => &mut self.stdout,
            Stream::Stderr => &mut self.stderr,
        }
    }

    fn failure(&mut self, path: &Path, error: &FileError) -> io::Result<()> {
        self.stdout.flush()?;
        let path = path.display();
        match error {
            FileError::Read(error) => writeln!(self.stderr, "oval: cannot read {path}: {error}"),
            FileError::InvalidUtf8 { line, column } => {
                writeln!(self.stderr, "oval: {path}:{line}:{column}: not valid UTF-8")
            }
            FileError::Rejected(rejection) => writeln!(self.stderr, "oval: {path}: {rejection}"),
            FileError::Write(error) => writeln!(self.stderr, "oval: cannot write {path}: {error}"),
            FileError::Serialize(error) => {
                writeln!(self.stderr, "oval: cannot serialize {path}: {error}")
            }
        }
    }
}

/// A one-based line and column, as `--json` output reports positions.
#[derive(Serialize)]
struct Position {
    line: usize,
    column: usize,
}

/// One diagnostic as `--json` output prints it.
#[derive(Serialize)]
struct Entry {
    path: String,
    #[serde(flatten)]
    diagnostic: Diagnostic,
    start: Position,
    end: Position,
    url: String,
}

/// Prints diagnostics to one stream, as text lines or, with `--json`, as one array at the end.
struct Printer {
    format: OutputFormat,
    stream: Stream,
    when_empty: WhenEmpty,
    entries: Vec<Entry>,
}

impl Printer {
    fn new(format: OutputFormat, stream: Stream, when_empty: WhenEmpty) -> Self {
        Self {
            format,
            stream,
            when_empty,
            entries: Vec::new(),
        }
    }

    fn report(
        &mut self,
        output: &mut Output,
        path: &Path,
        source: &str,
        report: &Report,
    ) -> io::Result<()> {
        let index = LineIndex::new(source);
        for diagnostic in report.diagnostics() {
            let (line, column) = index.line_column(diagnostic.range.start());
            match self.format {
                OutputFormat::Text => {
                    let severity = match diagnostic.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                    };
                    writeln!(
                        output.stream(self.stream),
                        "{}:{line}:{column}: {severity}[{}]: {}",
                        path.display(),
                        diagnostic.code,
                        diagnostic.message
                    )?;
                }
                OutputFormat::Json => {
                    let (end_line, end_column) = index.line_column(diagnostic.range.end());
                    self.entries.push(Entry {
                        path: path.display().to_string(),
                        diagnostic: diagnostic.clone(),
                        start: Position { line, column },
                        end: Position {
                            line: end_line,
                            column: end_column,
                        },
                        url: diagnostic.code.url(),
                    });
                }
            }
        }
        Ok(())
    }

    fn finish(self, output: &mut Output) -> io::Result<()> {
        let wanted = self.when_empty == WhenEmpty::PrintArray || !self.entries.is_empty();
        if self.format == OutputFormat::Json && wanted {
            let writer = output.stream(self.stream);
            serde_json::to_writer(&mut *writer, &self.entries)?;
            writeln!(writer)?;
        }
        Ok(())
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(status) => status.into(),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitStatus::Success.into(),
        Err(error) => {
            let _ = writeln!(io::stderr(), "oval: {error}");
            ExitStatus::Error.into()
        }
    }
}

fn run(cli: Cli) -> io::Result<ExitStatus> {
    let mut output = Output::new();
    let status = match cli.command {
        Command::Check {
            files,
            json,
            strict,
        } => check(
            &mut output,
            &files,
            OutputFormat::from_json_flag(json),
            strict,
        )?,
        Command::Compile { file, pretty, json } => compile(
            &mut output,
            &file,
            pretty,
            OutputFormat::from_json_flag(json),
        )?,
        Command::Fmt { files, check, json } => fmt(
            &mut output,
            &files,
            check,
            OutputFormat::from_json_flag(json),
        )?,
    };
    output.stdout.flush()?;
    Ok(status)
}

fn read_source(path: &Path) -> Result<String, FileError> {
    let bytes = std::fs::read(path).map_err(FileError::Read)?;
    String::from_utf8(bytes).map_err(|error| {
        let valid = &error.as_bytes()[..error.utf8_error().valid_up_to()];
        let prefix = std::str::from_utf8(valid).unwrap_or_default();
        let end = TextSize::try_from(prefix.len()).unwrap_or(TextSize::from(u32::MAX));
        let (line, column) = LineIndex::new(prefix).line_column(end);
        FileError::InvalidUtf8 { line, column }
    })
}

fn check(
    output: &mut Output,
    files: &[PathBuf],
    format: OutputFormat,
    strict: bool,
) -> io::Result<ExitStatus> {
    let mut status = ExitStatus::Success;
    let mut printer = Printer::new(format, Stream::Stdout, WhenEmpty::PrintArray);
    for path in files {
        let checked = read_source(path).and_then(|source| {
            let report = oval_core::check(&source).map_err(FileError::Rejected)?;
            Ok((source, report))
        });
        match checked {
            Ok((source, report)) => {
                printer.report(output, path, &source, &report)?;
                if report.has_errors() || (strict && report.has_diagnostics()) {
                    status = status.max(ExitStatus::Failure);
                }
            }
            Err(error) => {
                output.failure(path, &error)?;
                status = status.max(ExitStatus::Error);
            }
        }
    }
    printer.finish(output)?;
    Ok(status)
}

fn compile(
    output: &mut Output,
    path: &Path,
    pretty: bool,
    format: OutputFormat,
) -> io::Result<ExitStatus> {
    let source = match read_source(path) {
        Ok(source) => source,
        Err(error) => {
            output.failure(path, &error)?;
            return Ok(ExitStatus::Error);
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
                    writeln!(output.stdout, "{json}")?;
                    Ok(ExitStatus::Success)
                }
                Err(error) => {
                    output.failure(path, &FileError::Serialize(error))?;
                    Ok(ExitStatus::Error)
                }
            }
        }
        Err(CompileError::Rejected(rejection)) => {
            output.failure(path, &FileError::Rejected(rejection))?;
            Ok(ExitStatus::Error)
        }
        Err(CompileError::Invalid(report)) => {
            let mut printer = Printer::new(format, Stream::Stderr, WhenEmpty::PrintNothing);
            printer.report(output, path, &source, &report)?;
            printer.finish(output)?;
            Ok(ExitStatus::Failure)
        }
    }
}

fn fmt(
    output: &mut Output,
    files: &[PathBuf],
    check_only: bool,
    format: OutputFormat,
) -> io::Result<ExitStatus> {
    let mut status = ExitStatus::Success;
    let mut printer = Printer::new(format, Stream::Stderr, WhenEmpty::PrintNothing);
    for path in files {
        let source = match read_source(path) {
            Ok(source) => source,
            Err(error) => {
                output.failure(path, &error)?;
                status = status.max(ExitStatus::Error);
                continue;
            }
        };
        match oval_format::format(&source) {
            Ok(formatted) if formatted == source => {}
            Ok(_) if check_only => {
                writeln!(output.stdout, "{} would be reformatted", path.display())?;
                status = status.max(ExitStatus::Failure);
            }
            Ok(formatted) => {
                if let Err(error) = std::fs::write(path, formatted) {
                    output.failure(path, &FileError::Write(error))?;
                    status = status.max(ExitStatus::Error);
                }
            }
            Err(FormatError::Rejected(rejection)) => {
                output.failure(path, &FileError::Rejected(rejection))?;
                status = status.max(ExitStatus::Error);
            }
            Err(FormatError::SyntaxErrors(_)) => {
                if let Ok(report) = oval_core::check(&source) {
                    printer.report(output, path, &source, &report)?;
                }
                status = status.max(ExitStatus::Failure);
            }
        }
    }
    printer.finish(output)?;
    Ok(status)
}
