//! The `oval` command line tool for Oval voting guides.
//!
//! `oval check FILE...` parses and validates each file and prints every diagnostic, errors and
//! warnings alike, to standard output, one per line as
//! `file:line:column: severity[identifier]: message`. The line and column are one-based, and the
//! column counts characters. The identifier is the diagnostic's identifier in the specification,
//! such as `missing-guide-field`, and the message says what is wrong for a person to read.
//!
//! `oval compile FILE` compiles one file to its canonical JSON on standard output, compact or, with
//! `--pretty`, indented. Its diagnostics go to standard error in the same form, and a file with an
//! error prints no JSON.
//!
//! `oval fmt FILE...` rewrites each file in the canonical style, leaving a file that is already
//! formatted untouched. With `--check` it rewrites nothing, and prints the path of each file that
//! would change instead. A file with lexical or syntax errors is never formatted, and its errors go
//! to standard error in the same form as diagnostics.
//!
//! With `--json`, each command prints its diagnostics as one JSON array on the same stream instead,
//! with an object for each diagnostic whose keys are `path`, `code`, `severity`, `message`, `start`
//! and `end` as a one-based `line` and `column`, and `range` as the byte offsets `start` and `end`.
//! `check` prints the array even when it's empty, and `compile` and `fmt` only when it isn't.
//!
//! A file that can't be read or written, or that is rejected before lexing, is reported on standard
//! error in one line starting `oval:`, even with `--json`, and the remaining files are still
//! processed.
//!
//! The exit status is the worst outcome among the files. It's 0 on success, including when the
//! only diagnostics are warnings and `check` isn't given `--strict`. It's 1 when a file has an
//! error, or a warning under `check --strict`, or under `fmt --check` would change, and 2 when a
//! file can't be read or written or the command line is invalid. When standard
//! output is closed early, as by `head`, the run stops with status 0.

mod line_index;
mod printer;

use std::fmt;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oval_core::{InputRejection, Report};
use oval_format::FormatError;

use crate::printer::Printer;

#[derive(Parser)]
#[command(
    name = "oval",
    bin_name = "oval",
    version,
    about = "Tools for Oval voting guides"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Parse and validate one or more documents")]
    Check {
        #[arg(required = true, help = "The files to check")]
        files: Vec<PathBuf>,
        #[arg(long, help = "Fail when a file has warnings, not only errors")]
        strict: bool,
        #[arg(long, help = "Print the diagnostics as one JSON array")]
        json: bool,
    },
    #[command(about = "Compile a document to its canonical JSON")]
    Compile {
        #[arg(help = "The file to compile")]
        file: PathBuf,
        #[arg(long, help = "Indent the JSON for reading")]
        pretty: bool,
        #[arg(long, help = "Print the diagnostics as one JSON array")]
        json: bool,
    },
    #[command(about = "Rewrite one or more documents in the canonical style")]
    Fmt {
        #[arg(required = true, help = "The files to format")]
        files: Vec<PathBuf>,
        #[arg(long, help = "Rewrite nothing, and list the files that would change")]
        check: bool,
        #[arg(long, help = "Print the diagnostics as one JSON array")]
        json: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Status {
    Success,
    Failure,
    Error,
}

impl From<Status> for ExitCode {
    fn from(status: Status) -> Self {
        match status {
            Status::Success => ExitCode::SUCCESS,
            Status::Failure => ExitCode::FAILURE,
            Status::Error => ExitCode::from(2),
        }
    }
}

enum Problem {
    Unreadable(io::Error),
    Unwritable(io::Error),
    Rejected(InputRejection),
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(error) => error.fmt(f),
            Self::Unwritable(error) => {
                write!(f, "the formatted document can't be written: {error}")
            }
            Self::Rejected(rejection) => rejection.fmt(f),
        }
    }
}

impl Problem {
    fn report(&self, path: &Path) -> Status {
        eprintln!("oval: {}: {self}", path.display());
        Status::Error
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut output = BufWriter::new(io::stdout().lock());
    let result = match cli.command {
        Command::Check {
            files,
            strict,
            json,
        } => check(&files, strict, Printer::new(json), &mut output),
        Command::Compile { file, pretty, json } => {
            compile(&file, pretty, Printer::new(json), &mut output)
        }
        Command::Fmt { files, check, json } => {
            format(&files, check, Printer::new(json), &mut output)
        }
    }
    .and_then(|status| output.flush().map(|()| status));
    match result {
        Ok(status) => status.into(),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("oval: {error}");
            Status::Error.into()
        }
    }
}

fn check(
    files: &[PathBuf],
    strict: bool,
    mut printer: Printer,
    output: &mut impl Write,
) -> io::Result<Status> {
    let mut status = Status::Success;
    for path in files {
        let file_status = match load(path) {
            Ok((text, report)) => {
                printer.print(output, path, &text, report.diagnostics())?;
                let fails = report.has_errors() || strict && !report.diagnostics().is_empty();
                if fails {
                    Status::Failure
                } else {
                    Status::Success
                }
            }
            Err(problem) => problem.report(path),
        };
        status = status.max(file_status);
    }
    printer.finish(output, true)?;
    Ok(status)
}

fn compile(
    path: &Path,
    pretty: bool,
    mut printer: Printer,
    output: &mut impl Write,
) -> io::Result<Status> {
    let (text, report) = match load(path) {
        Ok(loaded) => loaded,
        Err(problem) => return Ok(problem.report(path)),
    };
    let mut errors = io::stderr().lock();
    printer.print(&mut errors, path, &text, report.diagnostics())?;
    printer.finish(&mut errors, false)?;
    let Some(compiled) = report.compile() else {
        return Ok(Status::Failure);
    };
    let json = if pretty {
        compiled.to_json_pretty()
    } else {
        compiled.to_json()
    };
    writeln!(output, "{json}")?;
    Ok(Status::Success)
}

fn format(
    files: &[PathBuf],
    check: bool,
    mut printer: Printer,
    output: &mut impl Write,
) -> io::Result<Status> {
    let mut status = Status::Success;
    for path in files {
        let text = match read(path) {
            Ok(text) => text,
            Err(problem) => {
                status = status.max(problem.report(path));
                continue;
            }
        };
        let file_status = match oval_format::format(&text) {
            Ok(formatted) if formatted == text => Status::Success,
            Ok(_) if check => {
                writeln!(output, "{}", path.display())?;
                Status::Failure
            }
            Ok(formatted) => match fs::write(path, formatted) {
                Ok(()) => Status::Success,
                Err(error) => Problem::Unwritable(error).report(path),
            },
            Err(FormatError::Rejected(rejection)) => Problem::Rejected(rejection).report(path),
            Err(FormatError::Malformed(parse)) => {
                let diagnostics = oval_core::parse_diagnostics(&parse);
                printer.print(&mut io::stderr().lock(), path, &text, &diagnostics)?;
                Status::Failure
            }
        };
        status = status.max(file_status);
    }
    printer.finish(&mut io::stderr().lock(), false)?;
    Ok(status)
}

fn read(path: &Path) -> Result<String, Problem> {
    let bytes = fs::read(path).map_err(Problem::Unreadable)?;
    String::from_utf8(bytes).map_err(|error| {
        Problem::Rejected(InputRejection::InvalidUtf8 {
            offset: u32::try_from(error.utf8_error().valid_up_to()).unwrap_or(u32::MAX),
        })
    })
}

fn load(path: &Path) -> Result<(String, Report), Problem> {
    let text = read(path)?;
    let report = oval_core::check(&text).map_err(Problem::Rejected)?;
    Ok((text, report))
}
