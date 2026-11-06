//! The `oval` command line tool.

mod line_index;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oval_core::{CompileError, Report, Severity};
use oval_format::FormatError;

use crate::line_index::LineIndex;

#[derive(Parser)]
#[command(name = "oval", version, about = "Tools for Oval voting guides")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and validate one or more documents.
    Check {
        /// Files to check.
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Compile a document to its canonical JSON form.
    Compile {
        /// The file to compile.
        file: PathBuf,
        /// Indent the output for reading.
        #[arg(long)]
        pretty: bool,
    },
    /// Rewrite one or more documents in the canonical style.
    Fmt {
        /// Files to format.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Report files that would change instead of rewriting them.
        #[arg(long)]
        check: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { files } => check(&files),
        Command::Compile { file, pretty } => compile(&file, pretty),
        Command::Fmt { files, check } => fmt(&files, check),
    }
}

fn fmt(files: &[PathBuf], check_only: bool) -> ExitCode {
    let mut worst = ExitCode::SUCCESS;
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
                    print_report(&mut std::io::stderr(), path, &source, &report);
                }
                worst = ExitCode::FAILURE;
            }
        }
    }
    worst
}

fn compile(path: &PathBuf, pretty: bool) -> ExitCode {
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
            print_report(&mut std::io::stderr(), path, &source, &report);
            ExitCode::FAILURE
        }
    }
}

fn check(files: &[PathBuf]) -> ExitCode {
    let mut worst = ExitCode::SUCCESS;
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
        print_report(&mut std::io::stdout(), path, &source, &report);
        if report.has_errors() {
            worst = ExitCode::FAILURE;
        }
    }
    worst
}

fn print_report(out: &mut impl Write, path: &std::path::Path, source: &str, report: &Report) {
    let index = LineIndex::new(source);
    for diagnostic in report.diagnostics() {
        let (line, column) = index.line_column(diagnostic.range.start());
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
