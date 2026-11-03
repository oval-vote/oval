//! The `oval` command line tool.

mod line_index;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oval_core::{Report, Severity};

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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { files } => check(&files),
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
        let report = oval_core::check(&source);
        print_report(path, &source, &report);
        if report.has_errors() {
            worst = ExitCode::FAILURE;
        }
    }
    worst
}

fn print_report(path: &std::path::Path, source: &str, report: &Report) {
    let index = LineIndex::new(source);
    for diagnostic in report.diagnostics() {
        let (line, column) = index.line_column(diagnostic.range.start());
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        println!(
            "{}:{line}:{column}: {severity}[{}]: {}",
            path.display(),
            diagnostic.code,
            diagnostic.message
        );
    }
}
