use clap::{Parser, Subcommand};
use oval_syntax::{has_errors, Diagnostic, Diagnostics, LineIndex, Severity};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "oval", version, about = "Tools for Oval Vote voting guides")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check files against the specification. Exits 1 if any file has errors.
    Check { files: Vec<PathBuf> },
    /// Check files and apply the lint rules (configured in oval.toml).
    Lint {
        files: Vec<PathBuf>,
        /// List the rules and their default levels instead.
        #[arg(long)]
        list: bool,
    },
    /// Rewrite files in canonical form.
    Fmt {
        files: Vec<PathBuf>,
        /// Do not write; exit 1 if any file would change.
        #[arg(long)]
        check: bool,
        /// Print the result instead of writing it (one file only).
        #[arg(long)]
        stdout: bool,
        /// Wrap prose at this width.
        #[arg(long, default_value_t = 80)]
        width: usize,
    },
    /// Print a file's data model as JSON.
    Export { file: PathBuf },
    /// Print a file's syntax tree (for debugging the parser).
    Tree { file: PathBuf },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Check { files } => check(&files, false),
        Command::Lint { files, list } => {
            if list {
                for r in oval_lint::RULES {
                    println!("{:<32} {:<6} {}", r.code, format!("{:?}", r.default).to_lowercase(), r.summary);
                }
                return ExitCode::SUCCESS;
            }
            check(&files, true)
        }
        Command::Fmt { files, check, stdout, width } => fmt(&files, check, stdout, width),
        Command::Export { file } => {
            let text = read(&file);
            let loaded = oval_model::load(&text);
            print_diagnostics(&file, &text, &loaded.diagnostics);
            if has_errors(&loaded.diagnostics) {
                return ExitCode::from(1);
            }
            println!("{}", serde_json::to_string_pretty(&loaded.guide).unwrap());
            ExitCode::SUCCESS
        }
        Command::Tree { file } => {
            let text = read(&file);
            print!("{:#?}", oval_syntax::parse(&text).syntax());
            ExitCode::SUCCESS
        }
    }
}

fn read(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            std::process::exit(2);
        }
    }
}

fn check(files: &[PathBuf], lint: bool) -> ExitCode {
    let (mut errors, mut warnings) = (0, 0);
    for file in files {
        let text = read(file);
        let loaded = oval_model::load(&text);
        let mut diagnostics = Diagnostics::from(loaded.diagnostics.clone());
        if lint {
            let (config, _) = match oval_lint::Config::discover(file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
            diagnostics.extend(oval_lint::lint(&loaded, &config));
            diagnostics.sort();
        }
        let diagnostics = diagnostics.into_vec();
        print_diagnostics(file, &text, &diagnostics);
        errors += diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
        warnings += diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
    }
    eprintln!("{} file(s), {errors} error(s), {warnings} warning(s)", files.len());
    if errors > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn fmt(files: &[PathBuf], check: bool, stdout: bool, width: usize) -> ExitCode {
    if stdout && files.len() != 1 {
        eprintln!("--stdout takes exactly one file");
        return ExitCode::from(2);
    }
    let opts = oval_fmt::Options { width };
    let mut status = ExitCode::SUCCESS;
    let mut changed = 0;
    for file in files {
        let text = read(file);
        match oval_fmt::format(&text, &opts) {
            Err(diagnostics) => {
                print_diagnostics(file, &text, &diagnostics);
                eprintln!("{}: not formatted; fix the syntax errors first", file.display());
                status = ExitCode::from(1);
            }
            Ok(out) if stdout => print!("{out}"),
            Ok(out) if out == text => {}
            Ok(out) => {
                changed += 1;
                if check {
                    eprintln!("{}: would be reformatted", file.display());
                    status = ExitCode::from(1);
                } else if let Err(e) = std::fs::write(file, out) {
                    eprintln!("{}: {e}", file.display());
                    status = ExitCode::from(2);
                }
            }
        }
    }
    if !stdout {
        eprintln!(
            "{} file(s), {changed} {}",
            files.len(),
            if check { "would change" } else { "reformatted" }
        );
    }
    status
}

fn print_diagnostics(file: &Path, text: &str, diagnostics: &[Diagnostic]) {
    let index = LineIndex::new(text);
    for d in diagnostics {
        let (line, col) = index.line_col(text, d.range.start());
        let sev = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        eprintln!("{}:{line}:{col}: {sev}[{}] {}", file.display(), d.code, d.message);
    }
}
