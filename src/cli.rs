//! The command-line host: argument parsing, I/O, and how a run is reported.
//!
//! Everything here is in the library rather than the binary, because logic
//! only the binary can reach is logic that cannot be tested.

use crate::reporting::{Mode, Reports, Settings, Severity};
use crate::values::Value;
use crate::vault::{self, Vault};
use crate::{render, transclude};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

/// How results are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Shape {
    /// For a person: the value on stdout, reports on stderr.
    Text,
    /// For a program: one JSON object carrying the value and the queue.
    Json,
}

/// HQL — Hyper Query Language.
#[derive(Debug, Parser)]
#[command(name = "hql", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    options: Options,
}

/// Options every subcommand accepts.
#[derive(Debug, Clone, Args)]
pub struct Options {
    /// The vault to run against; `cards` and traversal need one.
    #[arg(long, global = true, value_name = "DIR")]
    pub vault: Option<PathBuf>,
    /// How to write the result.
    #[arg(long, global = true, value_enum, default_value_t = Shape::Text)]
    pub format: Shape,
    /// When to stop: `strict` at the first error, `collect` to report them all.
    #[arg(long, global = true, value_name = "MODE")]
    pub report: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Type-check and evaluate one program given as an argument.
    Eval {
        /// The program.
        #[arg(allow_hyphen_values = true)]
        expression: String,
    },
    /// Parse and type-check a program without evaluating it.
    Check {
        /// A file, or `-` for standard input.
        file: String,
    },
    /// Type-check and evaluate a program from a file.
    Run {
        /// A file, or `-` for standard input.
        file: String,
    },
    /// Read, evaluate and print, one program at a time.
    Repl,
    /// Run the HQL blocks in a document and transclude each answer.
    Render {
        /// A Markdown or HyperMarkDown document.
        file: String,
        /// Write the rendered document back over the file.
        #[arg(long)]
        write: bool,
    },
    /// List the steps a pipeline may use.
    Builtins,
    /// Show the reporting mode in force, and where it came from.
    Config,
}

/// Run the CLI, returning the process exit code.
///
/// `0` is success, `1` is a language or I/O failure, and `2` is a usage error
/// that clap reports before this is reached.
#[must_use]
pub fn main(cli: Cli) -> u8 {
    let mut out = io::stdout();
    let mut err = io::stderr();
    match execute(&cli, &mut out, &mut err) {
        Ok(code) => code,
        Err(message) => {
            let _ = writeln!(err, "hql: {message}");
            1
        }
    }
}

/// Parse arguments from the process, for the binary.
#[must_use]
pub fn parse() -> Cli {
    Cli::parse()
}

fn execute(cli: &Cli, out: &mut impl Write, err: &mut impl Write) -> Result<u8, String> {
    let vault = load_vault(cli.options.vault.as_deref())?;
    let requested = match &cli.options.report {
        Some(name) => Some(Mode::parse(name).ok_or_else(|| {
            format!("unknown report mode `{name}`; expected `strict` or `collect`")
        })?),
        None => None,
    };
    let settings = Settings::resolve(requested, cli.options.vault.as_deref());

    match &cli.command {
        Command::Builtins => {
            for provider in crate::extensions::catalogue(&vault.extensions) {
                writeln!(
                    out,
                    "{} {} ({})",
                    provider.name,
                    provider.version,
                    provider.availability.label()
                )
                .map_err(|error| error.to_string())?;
                for step in provider.steps {
                    writeln!(
                        out,
                        "  {}\n    {}\n    {}",
                        step.name, step.signature, step.summary
                    )
                    .map_err(|error| error.to_string())?;
                }
            }
            Ok(0)
        }
        Command::Config => {
            writeln!(
                out,
                "report mode: {} (from {})\nvault: {}",
                settings.mode.label(),
                settings.origin,
                if vault.is_present() {
                    format!("{} ({} cards)", vault.root.display(), vault.cards.len())
                } else {
                    "none".to_owned()
                }
            )
            .map_err(|error| error.to_string())?;
            Ok(0)
        }
        Command::Eval { expression } => {
            let outcome = crate::run(expression, &vault, settings.mode);
            emit(&outcome, expression, "<expression>", &cli.options, out, err)
        }
        Command::Run { file } => {
            let (source, origin) = read(file)?;
            let outcome = crate::run(&source, &vault, settings.mode);
            emit(&outcome, &source, &origin, &cli.options, out, err)
        }
        Command::Check { file } => {
            let (source, origin) = read(file)?;
            let outcome = crate::check_reported(&source, &vault, settings.mode);
            emit(&outcome, &source, &origin, &cli.options, out, err)
        }
        Command::Render { file, write } => {
            let (source, origin) = read(file)?;
            let rendered = transclude::render(&source, &vault, settings.mode);
            if *write {
                if file == "-" {
                    return Err("--write needs a file, not standard input".to_owned());
                }
                fs::write(file, &rendered.document).map_err(|error| format!("{file}: {error}"))?;
                writeln!(
                    out,
                    "{file}: {} block{} rendered",
                    rendered.blocks,
                    if rendered.blocks == 1 { "" } else { "s" }
                )
                .map_err(|error| error.to_string())?;
            } else {
                write!(out, "{}", rendered.document).map_err(|error| error.to_string())?;
            }
            report_all(&rendered.reports, &source, &origin, err)?;
            Ok(u8::from(rendered.reports.has_errors()))
        }
        Command::Repl => repl(&vault, settings.mode, &cli.options, out, err),
    }
}

fn load_vault(root: Option<&Path>) -> Result<Vault, String> {
    match root {
        None => Ok(Vault::empty()),
        Some(root) => {
            if !root.is_dir() {
                return Err(format!("{}: not a directory", root.display()));
            }
            vault::load(root).map_err(|error| format!("{}: {error}", root.display()))
        }
    }
}

/// Read a program from a file, or from standard input when named `-`.
fn read(file: &str) -> Result<(String, String), String> {
    if file == "-" {
        let mut source = String::new();
        io::Read::read_to_string(&mut io::stdin(), &mut source)
            .map_err(|error| format!("<stdin>: {error}"))?;
        return Ok((source, "<stdin>".to_owned()));
    }
    let source = fs::read_to_string(file).map_err(|error| format!("{file}: {error}"))?;
    Ok((source, file.to_owned()))
}

fn emit(
    outcome: &crate::Outcome,
    source: &str,
    origin: &str,
    options: &Options,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<u8, String> {
    match options.format {
        Shape::Json => {
            let document = serde_json::json!({
                "origin": origin,
                "type": outcome.inferred.as_ref().map(std::string::ToString::to_string),
                "value": outcome.value.as_ref().map(Value::to_json),
                "reports": outcome.reports.to_json(),
            });
            writeln!(
                out,
                "{}",
                serde_json::to_string_pretty(&document).unwrap_or_else(|_| "null".to_owned())
            )
            .map_err(|error| error.to_string())?;
        }
        Shape::Text => {
            if let Some(value) = &outcome.value {
                writeln!(out, "{value}").map_err(|error| error.to_string())?;
            } else if outcome.value.is_none()
                && !outcome.failed()
                && let Some(inferred) = &outcome.inferred
            {
                // `check` has a type and no value; that is the whole answer.
                writeln!(out, "{inferred}").map_err(|error| error.to_string())?;
            }
            report_all(&outcome.reports, source, origin, err)?;
        }
    }
    Ok(u8::from(outcome.failed()))
}

/// Write the queue, oldest first, with a summary when there is more than one.
fn report_all(
    reports: &Reports,
    source: &str,
    origin: &str,
    err: &mut impl Write,
) -> Result<(), String> {
    for report in reports.iter() {
        writeln!(err, "{}\n", render::report(report, source, origin))
            .map_err(|error| error.to_string())?;
    }
    if reports.len() > 1 {
        writeln!(
            err,
            "{} report{}: {} failure, {} error, {} warning",
            reports.len(),
            if reports.len() == 1 { "" } else { "s" },
            reports.count(Severity::Failure),
            reports.count(Severity::Error),
            reports.count(Severity::Warning),
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Read, evaluate and print, keeping bindings across lines.
///
/// Bindings persist by replaying the statements that succeeded, which keeps
/// the session a program rather than a sequence of unrelated runs.
fn repl(
    vault: &Vault,
    mode: Mode,
    options: &Options,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<u8, String> {
    let interactive = io::stdin().is_terminal();
    if interactive {
        writeln!(
            out,
            "hql {}. `:help` for commands, `:quit` to leave.",
            env!("CARGO_PKG_VERSION")
        )
        .map_err(|error| error.to_string())?;
    }
    let mut prelude: Vec<String> = Vec::new();
    let stdin = io::stdin();
    loop {
        if interactive {
            write!(out, "hql> ").map_err(|error| error.to_string())?;
            out.flush().map_err(|error| error.to_string())?;
        }
        let mut line = String::new();
        if stdin
            .lock()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?
            == 0
        {
            return Ok(0);
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line {
            ":quit" | ":q" => return Ok(0),
            ":help" | ":h" => {
                writeln!(
                    out,
                    ":type <expression>   the type, without evaluating\n\
                     :vault               what the session is running against\n\
                     :reset               forget the bindings made so far\n\
                     :quit                leave"
                )
                .map_err(|error| error.to_string())?;
                continue;
            }
            ":vault" => {
                writeln!(
                    out,
                    "{}",
                    if vault.is_present() {
                        format!("{} ({} cards)", vault.root.display(), vault.cards.len())
                    } else {
                        "no vault".to_owned()
                    }
                )
                .map_err(|error| error.to_string())?;
                continue;
            }
            ":reset" => {
                prelude.clear();
                continue;
            }
            _ => {}
        }

        let (source, typing) = match line.strip_prefix(":type ") {
            Some(rest) => (join(&prelude, rest), true),
            None => (join(&prelude, line), false),
        };
        let outcome = if typing {
            crate::check_reported(&source, vault, mode)
        } else {
            crate::run(&source, vault, mode)
        };
        emit(&outcome, &source, "<repl>", options, out, err)?;
        // A binding that worked joins the session; an expression does not.
        if !outcome.failed() && !typing && is_binding(line) {
            prelude.push(line.to_owned());
        }
    }
}

fn join(prelude: &[String], line: &str) -> String {
    let mut source = prelude.join("\n");
    if !source.is_empty() {
        source.push('\n');
    }
    source.push_str(line);
    source
}

/// Whether a line binds a name, and so belongs to the session.
fn is_binding(line: &str) -> bool {
    let Some((head, _)) = line.split_once('=') else {
        return false;
    };
    let head =
        head.trim_end_matches(|c: char| c == ':' || c.is_whitespace() || c.is_alphanumeric());
    head.is_empty() && !line.starts_with('=') && !line.contains("=>")
}
