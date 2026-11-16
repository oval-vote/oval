//! The `oval` command line tool for Oval voting guides.
//!
//! `oval check FILE...` parses and validates each file and prints every diagnostic, errors and
//! warnings alike, to standard output, one per line as
//! `file:line:column: severity[identifier]: message`. The line and column are one-based, and the
//! column counts characters. The identifier is the diagnostic's identifier in the specification,
//! such as `missing-guide-field`, and the message says what is wrong for a person to read.
//!
//! A file that can't be read, or that is rejected before lexing, is reported on standard error in
//! one line starting `oval:`, and the remaining files are still checked.
//!
//! The exit status is the worst outcome among the files. It's 0 when no file has an error, 1 when
//! a file has an error, and 2 when a file can't be checked or the command line is invalid. When
//! standard output is closed early, as by `head`, the run stops with status 0.

mod line_index;

use std::fmt;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oval_core::{InputRejection, Report, Severity};

use crate::line_index::LineIndex;

#[derive(Parser)]
#[command(name = "oval", version, about = "Tools for Oval voting guides")]
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
    Rejected(InputRejection),
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(error) => error.fmt(f),
            Self::Rejected(rejection) => rejection.fmt(f),
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut output = BufWriter::new(io::stdout().lock());
    let result = match cli.command {
        Command::Check { files } => check(&files, &mut output),
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

fn check(files: &[PathBuf], output: &mut impl Write) -> io::Result<Status> {
    let mut status = Status::Success;
    for path in files {
        let file_status = match load(path) {
            Ok((text, report)) => print(output, path, &text, &report)?,
            Err(problem) => {
                eprintln!("oval: {}: {problem}", path.display());
                Status::Error
            }
        };
        status = status.max(file_status);
    }
    Ok(status)
}

fn load(path: &Path) -> Result<(String, Report), Problem> {
    let bytes = fs::read(path).map_err(Problem::Unreadable)?;
    let text = String::from_utf8(bytes).map_err(|error| {
        Problem::Rejected(InputRejection::InvalidUtf8 {
            offset: u32::try_from(error.utf8_error().valid_up_to()).unwrap_or(u32::MAX),
        })
    })?;
    let report = oval_core::check(&text).map_err(Problem::Rejected)?;
    Ok((text, report))
}

fn print(output: &mut impl Write, path: &Path, text: &str, report: &Report) -> io::Result<Status> {
    let index = LineIndex::new(text);
    for diagnostic in report.diagnostics() {
        let (line, column) = index.position(diagnostic.range.start().into());
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        writeln!(
            output,
            "{}:{line}:{column}: {severity}[{}]: {}",
            path.display(),
            diagnostic.code.spec_id(),
            diagnostic.message
        )?;
    }
    Ok(if report.has_errors() {
        Status::Failure
    } else {
        Status::Success
    })
}
