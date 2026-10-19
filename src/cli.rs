//! The command-line host: argument parsing, I/O, and how a run is reported.
//!
//! Everything here is in the library rather than the binary, because logic
//! only the binary can reach is logic that cannot be tested.

use crate::lexer::Token;
use crate::reporting::{Level, Mode, Reports, Settings, Severity};
use crate::types::{TypeConstructor, Value};
use crate::vault::{self, Vault};
use crate::{render, transclude};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::FileHistory;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Editor, Helper};
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

/// How results are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Shape {
    /// The value alone on stdout, written in the `--to` format; reports on
    /// stderr.
    Text,
    /// One JSON object carrying the type, the value and the reports.
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
    /// What the answer is wrapped in: the value alone, or one JSON document
    /// carrying the type, the value and the reports. How the value itself is
    /// written is `--to`.
    #[arg(long, global = true, value_enum, default_value_t = Shape::Text)]
    pub envelope: Shape,
    /// The presentation format a result is written in when the program did
    /// not format it: `hql` (the default), `text`, `markdown`, `hmd`, `json`,
    /// `yaml`, `csv`, `d2` or `mermaid`; `none` evaluates and writes no value.
    #[arg(long, global = true, value_name = "FORMAT")]
    pub to: Option<String>,
    /// Write each JSON document of a result on one line, as `jq -c` does; a
    /// stream is then JSON Lines.
    #[arg(long, global = true)]
    pub compact: bool,
    /// Every logger's threshold, overriding the one it was constructed with:
    /// `trace`, `debug`, `info`, `warn` or `error`.
    #[arg(long, global = true, value_name = "LEVEL")]
    pub log_level: Option<String>,
    /// When to stop: `strict` at the first error, `collect` to report them all.
    #[arg(long, global = true, value_name = "MODE")]
    pub report: Option<String>,
    /// The lowest severity shown: `debug`, `info`, `warning` or `error`.
    /// `--envelope json` carries every entry whatever this says.
    #[arg(long, global = true, value_name = "LEVEL")]
    pub report_level: Option<String>,
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
    /// Run the HQL cells in a document and write each one's output.
    Render {
        /// A Markdown or HyperMarkDown document, or `-` for standard input.
        file: String,
        /// Write the rendered document to this file instead of standard output.
        #[arg(long, value_name = "OUT", conflicts_with = "in_place")]
        write: Option<PathBuf>,
        /// Write the rendered document over the input. Needs `--emit after`,
        /// since `replace` would remove the cells from the source.
        #[arg(long)]
        in_place: bool,
        /// Where a cell's output goes: in the cell's place, or after it.
        #[arg(long, value_enum, default_value_t = Emit::Replace)]
        emit: Emit,
        /// Run this cell and the cells it reads, and rewrite only its output.
        /// Needs `--emit after`, since a build is of the whole document.
        #[arg(long, value_name = "ID")]
        cell: Option<String>,
    },
    /// List the steps a pipeline may use.
    Builtins,
    /// Show the reporting settings in force, and where each came from.
    Config,
    /// Check the semantic-search chain — vault, configuration, index, model,
    /// query set — and name the first part that is missing.
    Doctor,
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

/// Where `hql render` writes a cell's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Emit {
    /// In the cell's place: the rendered document is a build of the source.
    Replace,
    /// After the cell, which stays: the document stays a source.
    After,
}

impl From<Emit> for transclude::Emit {
    fn from(emit: Emit) -> Self {
        match emit {
            Emit::Replace => Self::Replace,
            Emit::After => Self::After,
        }
    }
}

/// The directory a source unit's relative paths resolve against: the file's
/// own, or the working directory for standard input.
fn directory_of(file: &str) -> PathBuf {
    if file == "-" {
        return PathBuf::from(".");
    }
    Path::new(file)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// The commands a first argument may name; anything else is a program.
const COMMANDS: [&str; 9] = [
    "eval", "check", "run", "repl", "render", "builtins", "config", "doctor", "help",
];

/// Options that take a value, so the word after one is not the program.
const VALUED: [&str; 6] = [
    "--vault",
    "--envelope",
    "--to",
    "--report",
    "--report-level",
    "--log-level",
];

/// `hql '<program>'` is `hql eval '<program>'`: insert `eval` before a first
/// word that names no command.
#[must_use]
pub fn with_default_command(arguments: Vec<String>) -> Vec<String> {
    let mut index = 1;
    while let Some(argument) = arguments.get(index) {
        if VALUED.contains(&argument.as_str()) {
            index += 2;
            continue;
        }
        if argument.starts_with("--") || (argument.starts_with('-') && argument.len() > 1) {
            index += 1;
            continue;
        }
        if COMMANDS.contains(&argument.as_str()) {
            return arguments;
        }
        let mut arguments = arguments;
        arguments.insert(index, "eval".to_owned());
        return arguments;
    }
    arguments
}

/// Parse arguments from the process, for the binary.
///
/// A combination clap cannot express is still a usage error, exit `2`.
#[must_use]
pub fn parse() -> Cli {
    let cli = Cli::parse_from(with_default_command(std::env::args().collect()));
    if let Command::Render {
        in_place,
        emit,
        cell,
        ..
    } = &cli.command
        && *emit == Emit::Replace
    {
        let refused = if *in_place {
            Some(
                "--in-place needs --emit after: replacing the cells would remove them from the source",
            )
        } else if cell.is_some() {
            Some("--cell needs --emit after: a build is of the whole document")
        } else {
            None
        };
        if let Some(message) = refused {
            use clap::CommandFactory;
            Cli::command()
                .error(clap::error::ErrorKind::ArgumentConflict, message)
                .exit();
        }
    }
    cli
}

fn execute(cli: &Cli, out: &mut impl Write, err: &mut impl Write) -> Result<u8, String> {
    let vault = load_vault(cli.options.vault.as_deref())?;
    if let Some(level) = &cli.options.log_level
        && crate::extensions::logging::Level::parse(level).is_none()
    {
        return Err(format!(
            "unknown log level `{level}`; expected `trace`, `debug`, `info`, `warn` or `error`"
        ));
    }
    crate::io::set_log_level(cli.options.log_level.clone());
    let requested = match &cli.options.report {
        Some(name) => Some(Mode::parse(name).ok_or_else(|| {
            format!("unknown report mode `{name}`; expected `strict` or `collect`")
        })?),
        None => None,
    };
    let settings = Settings::resolve(requested, cli.options.vault.as_deref());
    let level = match &cli.options.report_level {
        Some(name) => Some(Severity::parse_level(name).ok_or_else(|| {
            format!("unknown report level `{name}`; expected `debug`, `info`, `warning` or `error`")
        })?),
        None => None,
    };
    let level = Level::resolve(level, cli.options.vault.as_deref());
    let shown = level.severity;

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
                        step.name,
                        step.signature.replace('\n', "\n    "),
                        step.summary
                    )
                    .map_err(|error| error.to_string())?;
                }
            }
            Ok(0)
        }
        Command::Doctor => {
            let checks = crate::extensions::semantic::doctor(&vault);
            for check in &checks {
                writeln!(
                    out,
                    "{:<8} {:<7} {}",
                    check.part,
                    if check.ok { "ok" } else { "missing" },
                    check.detail
                )
                .map_err(|error| error.to_string())?;
            }
            Ok(u8::from(checks.iter().any(|check| !check.ok)))
        }
        Command::Config => {
            writeln!(
                out,
                "report mode: {} (from {})\nreport level: {} (from {})\nvault: {}",
                settings.mode.label(),
                settings.origin,
                level.severity.label(),
                level.origin,
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
            crate::io::set_host(crate::io::Policy::Open, PathBuf::from("."));
            let outcome = crate::run(expression, &vault, settings.mode);
            emit(
                &outcome,
                expression,
                "<expression>",
                HQL,
                &cli.options,
                &vault,
                shown,
                out,
                err,
            )
        }
        Command::Run { file } => {
            let (source, origin) = read(file)?;
            crate::io::set_host(crate::io::Policy::Open, directory_of(file));
            let outcome = crate::run(&source, &vault, settings.mode);
            emit(
                &outcome,
                &source,
                &origin,
                HQL,
                &cli.options,
                &vault,
                shown,
                out,
                err,
            )
        }
        Command::Check { file } => {
            let (source, origin) = read(file)?;
            let outcome = crate::check_reported(&source, &vault, settings.mode);
            emit(
                &outcome,
                &source,
                &origin,
                TEXT,
                &cli.options,
                &vault,
                shown,
                out,
                err,
            )
        }
        Command::Render {
            file,
            write,
            in_place,
            emit,
            cell,
        } => {
            let (source, origin) = read(file)?;
            // A document reads beside itself and writes nothing outside.
            let root = directory_of(file);
            crate::io::set_host(crate::io::Policy::Document { root: root.clone() }, root);
            let rendered = transclude::render_cell(
                &source,
                &vault,
                settings.mode,
                (*emit).into(),
                cell.as_deref(),
            )?;
            if cli.options.envelope == Shape::Json {
                let cells: Vec<_> = rendered
                    .cells
                    .iter()
                    .map(|cell| {
                        serde_json::json!({
                            "id": cell.id,
                            "index": cell.index,
                            "span": [cell.span.start, cell.span.end],
                            "hash": cell.hash,
                            "type": cell.ty,
                            "status": cell.status.label(),
                            "markdown": cell.markdown,
                            "reports": cell.reports.to_json(),
                        })
                    })
                    .collect();
                let document = serde_json::json!({
                    "document": rendered.document,
                    "cells": cells,
                    "reports": rendered.reports.to_json(),
                });
                writeln!(
                    out,
                    "{}",
                    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "null".to_owned())
                )
                .map_err(|error| error.to_string())?;
                return Ok(u8::from(
                    rendered.reports.has_errors() || !rendered.complete,
                ));
            }
            report_all(&rendered.reports.shown(shown), &source, &origin, err)?;
            // A build with a failed cell in it is not written anywhere.
            if !rendered.complete {
                return Ok(1);
            }
            let target = match (write, in_place) {
                (Some(path), _) => Some(path.clone()),
                (None, true) if file == "-" => {
                    return Err("--in-place needs a file, not standard input".to_owned());
                }
                (None, true) => Some(PathBuf::from(file)),
                (None, false) => None,
            };
            match target {
                Some(path) => {
                    fs::write(&path, &rendered.document)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    writeln!(
                        out,
                        "{}: {} block{} rendered",
                        path.display(),
                        rendered.blocks,
                        if rendered.blocks == 1 { "" } else { "s" }
                    )
                    .map_err(|error| error.to_string())?;
                }
                None => write!(out, "{}", rendered.document).map_err(|error| error.to_string())?,
            }
            Ok(u8::from(rendered.reports.has_errors()))
        }
        Command::Repl => repl(&vault, settings.mode, shown, &cli.options, out, err),
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

/// The format a result is written in when neither the program nor `--to`
/// chose one: HQL literals for a value, plain text for what `check` writes,
/// a type.
const HQL: TypeConstructor = TypeConstructor("Hql");
const TEXT: TypeConstructor = TypeConstructor("Text");

/// What `--to` asks for: no value at all, or a presentation format.
enum Presentation {
    Nothing,
    Format(crate::types::TypeConstructor),
}

fn presentation(to: Option<&str>) -> Result<Option<Presentation>, String> {
    match to {
        None => Ok(None),
        Some("none") => Ok(Some(Presentation::Nothing)),
        Some(name) => crate::extensions::format::format_named(name)
            .map(|format| Some(Presentation::Format(format)))
            .ok_or_else(|| {
                format!(
                    "unknown format `{name}` for --to; expected `none` or one of {}",
                    crate::extensions::format::FORMATS
                        .iter()
                        .map(|(_, written)| format!("`{written}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }),
    }
}

/// A result written in the presentation context: as the program formatted
/// it, else in the format `--to` names, else as text.
fn presented(
    value: &Value,
    to: Option<&Presentation>,
    host: TypeConstructor,
    vault: &Vault,
    compact: bool,
) -> Result<Option<String>, String> {
    // A program that ends in a write has said everything it had to.
    if matches!(value, Value::Unit) {
        return Ok(None);
    }
    let format = match to {
        Some(Presentation::Nothing) => return Ok(None),
        Some(Presentation::Format(format)) => *format,
        None => match value {
            Value::Formatted(formatted) => return Ok(Some(formatted.text.clone())),
            _ => host,
        },
    };
    if compact && format.0 == "Json" {
        let settled = value.settled();
        let json = settled.as_ref().unwrap_or(value).to_json();
        return serde_json::to_string(&json)
            .map(Some)
            .map_err(|error| error.to_string());
    }
    crate::extensions::format::render(value, format, 0, None, &vault.formats).map(Some)
}

/// Write a stream as it arrives, one piece per value, each flushed so a
/// reader sees it at once. A reader that closes the pipe ends the stream.
fn streamed(
    traversal: &crate::traversal::Traversal,
    to: Option<&Presentation>,
    host: TypeConstructor,
    vault: &Vault,
    compact: bool,
    out: &mut impl Write,
) -> Result<(), String> {
    let format = match to {
        Some(Presentation::Format(format)) => *format,
        _ => host,
    };
    let (mut first, mut failed) = (true, None);
    crate::stream(traversal, vault, |value| {
        let text = match (&value, to) {
            (Value::Formatted(formatted), None) => Ok(formatted.text.clone()),
            // Each document pretty-printed, one after another, as `jq .`
            // writes them; concatenated JSON reads back as the same stream.
            _ if format.0 == "Json" && !compact => {
                serde_json::to_string_pretty(&value.to_json()).map_err(|error| error.to_string())
            }
            _ => crate::extensions::format::piece(&value, format, first),
        };
        first = false;
        match text {
            Ok(text) => writeln!(out, "{text}").and_then(|()| out.flush()).is_ok(),
            Err(message) => {
                failed = Some(message);
                false
            }
        }
    })
    .map_err(|diagnostic| diagnostic.message())?;
    failed.map_or(Ok(()), Err)
}

#[allow(clippy::too_many_arguments)]
fn emit(
    outcome: &crate::Outcome,
    source: &str,
    origin: &str,
    host: TypeConstructor,
    options: &Options,
    vault: &Vault,
    shown: Severity,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<u8, String> {
    let to = presentation(options.to.as_deref())?;
    if let Some(Value::Traversal(traversal)) = &outcome.value
        && !traversal.finite
    {
        if options.envelope == Shape::Json {
            return Err(format!(
                "--envelope json writes one document, and {} may not end; bound it with `take(n)`",
                traversal.type_of()
            ));
        }
        if !matches!(to, Some(Presentation::Nothing)) {
            streamed(traversal, to.as_ref(), host, vault, options.compact, out)?;
        }
        report_all(&outcome.reports.shown(shown), source, origin, err)?;
        return Ok(u8::from(outcome.failed()));
    }
    match options.envelope {
        Shape::Json => {
            // The value is data unless `--to` asked for a format.
            let value = match (&outcome.value, &to) {
                (Some(_), Some(Presentation::Nothing)) | (None, _) => serde_json::Value::Null,
                (Some(value), Some(_)) => {
                    presented(value, to.as_ref(), host, vault, options.compact)?
                        .map_or(serde_json::Value::Null, serde_json::Value::String)
                }
                (Some(value), None) => value.to_json(),
            };
            let document = serde_json::json!({
                "origin": origin,
                "type": outcome.inferred.as_ref().map(std::string::ToString::to_string),
                "value": value,
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
                if let Some(text) = presented(value, to.as_ref(), host, vault, options.compact)? {
                    writeln!(out, "{text}").map_err(|error| error.to_string())?;
                }
            } else if outcome.value.is_none()
                && !outcome.failed()
                && let Some(inferred) = &outcome.inferred
            {
                for signature in outcome
                    .reports
                    .iter()
                    .filter(|report| matches!(report.code.as_deref(), Some("signature" | "effect")))
                {
                    writeln!(out, "{}", signature.message).map_err(|error| error.to_string())?;
                }
                // `check` has a type and no value; that is the whole answer.
                writeln!(out, "{inferred}").map_err(|error| error.to_string())?;
            }
            report_all(&outcome.reports.shown(shown), source, origin, err)?;
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
        let mut summary = format!(
            "{} report{}: {} failure, {} error, {} warning",
            reports.len(),
            if reports.len() == 1 { "" } else { "s" },
            reports.count(Severity::Failure),
            reports.count(Severity::Error),
            reports.count(Severity::Warning),
        );
        for severity in [Severity::Info, Severity::Debug] {
            let count = reports.count(severity);
            if count > 0 {
                summary.push_str(&format!(", {count} {}", severity.label()));
            }
        }
        writeln!(err, "{summary}").map_err(|error| error.to_string())?;
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
    shown: Severity,
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
    // The last input that ran, so a line opening with `|` continues it, and
    // whether it was a binding the session kept.
    let mut last: Option<(String, bool)> = None;
    let mut input = if interactive {
        Input::terminal()?
    } else {
        Input::Piped(io::stdin().lock())
    };
    loop {
        let Some(line) = input.next()? else {
            input.close();
            return Ok(0);
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line {
            ":quit" | ":q" => {
                input.close();
                return Ok(0);
            }
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

        // A line opening with `|` continues the pipeline before it: the
        // two run as one input, and a binding is made again with both.
        let mut replaced = None;
        let continued;
        let line = match (line.starts_with('|'), &last) {
            (true, Some((previous, kept))) => {
                if *kept {
                    replaced = prelude.pop();
                }
                continued = format!("{previous}\n{line}");
                continued.as_str()
            }
            _ => line,
        };
        let (source, typing) = match line.strip_prefix(":type ") {
            Some(rest) => (join(&prelude, rest), true),
            None => (join(&prelude, line), false),
        };
        let outcome = if typing {
            crate::check_reported(&source, vault, mode)
        } else {
            crate::run(&source, vault, mode)
        };
        emit(
            &outcome, &source, "<repl>", HQL, options, vault, shown, out, err,
        )?;
        // A binding that worked joins the session; an expression does not.
        // A continuation that failed leaves the binding it continued.
        let kept = !outcome.failed() && !typing && is_binding(line);
        if kept {
            prelude.push(line.to_owned());
        } else if let Some(held) = replaced {
            prelude.push(held);
        }
        if !typing && !outcome.failed() {
            last = Some((line.to_owned(), kept));
        }
    }
}

/// Where the REPL reads its inputs: a line editor on a terminal, or plain
/// lines from a pipe.
enum Input {
    Terminal(Box<Editor<Continuation, FileHistory>>, Option<PathBuf>),
    Piped(io::StdinLock<'static>),
}

impl Input {
    fn terminal() -> Result<Self, String> {
        let mut editor =
            Editor::<Continuation, FileHistory>::new().map_err(|error| error.to_string())?;
        editor.set_helper(Some(Continuation));
        let history = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".hql_history"));
        if let Some(path) = &history {
            // A first session has no history yet; that is not a failure.
            let _ = editor.load_history(path);
        }
        Ok(Self::Terminal(Box::new(editor), history))
    }

    /// The next whole input, which spans lines while a bracket is open, or
    /// `None` at the end of input.
    fn next(&mut self) -> Result<Option<String>, String> {
        match self {
            Self::Terminal(editor, _) => match editor.readline("hql> ") {
                Ok(text) => {
                    if !text.trim().is_empty() {
                        let _ = editor.add_history_entry(text.as_str());
                    }
                    Ok(Some(text))
                }
                // Ctrl-C abandons the input being typed, not the session.
                Err(ReadlineError::Interrupted) => Ok(Some(String::new())),
                Err(ReadlineError::Eof) => Ok(None),
                Err(error) => Err(error.to_string()),
            },
            Self::Piped(stdin) => {
                let mut text = String::new();
                loop {
                    let read = stdin
                        .read_line(&mut text)
                        .map_err(|error| error.to_string())?;
                    if read == 0 {
                        return Ok((!text.is_empty()).then_some(text));
                    }
                    // A pipeline written one step per line, `| sort`, goes on
                    // while the next line opens with `|`.
                    if !is_open(&text) && !continues(stdin) {
                        return Ok(Some(text));
                    }
                }
            }
        }
    }

    fn close(&mut self) {
        if let Self::Terminal(editor, Some(path)) = self {
            // Losing the history must not turn a clean exit into a failure.
            let _ = editor.save_history(path);
        }
    }
}

/// The line editor's helper: it holds an input open while a bracket is.
struct Continuation;

impl Validator for Continuation {
    fn validate(&self, context: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(if is_open(context.input()) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Valid(None)
        })
    }
}

impl Completer for Continuation {
    type Candidate = String;
}

impl Hinter for Continuation {
    type Hint = String;
}

impl Highlighter for Continuation {}

impl Helper for Continuation {}

/// Whether the next piped line opens with `|`, continuing the input before it.
fn continues(stdin: &mut io::StdinLock<'static>) -> bool {
    stdin.fill_buf().is_ok_and(|ahead| {
        ahead
            .iter()
            .find(|byte| !matches!(byte, b' ' | b'\t'))
            .is_some_and(|byte| *byte == b'|')
    })
}

/// Whether an input leaves a bracket open, and so continues on the next line.
///
/// A string literal cannot span lines, so brackets alone decide; input that
/// does not scan is complete, and its diagnostic is reported when it runs.
fn is_open(source: &str) -> bool {
    let Ok(tokens) = crate::lexer::scan(source) else {
        return false;
    };
    let depth = tokens
        .iter()
        .fold(0_i64, |depth, spanned| match spanned.token {
            Token::OpenParen | Token::OpenBracket | Token::OpenBrace => depth + 1,
            Token::CloseParen | Token::CloseBracket | Token::CloseBrace => depth - 1,
            _ => depth,
        });
    depth > 0
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
    // An input the session keeps declares and evaluates nothing of its own:
    // bindings, `fn`, aliases, `type` and `import`, but no expression.
    crate::parser::parse(line).is_ok_and(|program| {
        program
            .statements
            .iter()
            .all(|statement| !matches!(statement, crate::ast::Stmt::Expr(_)))
    })
}
