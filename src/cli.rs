//! The command-line host: argument parsing, I/O, and how a run is reported.
//!
//! Everything here is in the library rather than the binary, because logic
//! only the binary can reach is logic that cannot be tested.

use crate::diagnostics::Diagnostic;
use crate::lexer::Token;
use crate::reporting::{Level, Mode, Settings, Severity};
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
    /// The value alone on stdout, written in the `--to` format; diagnostics
    /// on stderr.
    Text,
    /// One JSON object carrying the type, the value and the diagnostics.
    Json,
}

/// The format the run's diagnostics are rendered in on their target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagnosticsFormat {
    /// `DiagnosticText`: a heading, the source line and a caret, then each
    /// field that applies.
    Text,
    /// One JSON document per diagnostic, one per line, every field that
    /// applies present.
    Json,
}

/// HQL — Hyper Query Language.
#[derive(Debug, Parser)]
#[command(name = "hql", version, about, long_about = None, disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    options: Options,
}

/// Options every subcommand accepts.
#[derive(Debug, Clone, Args)]
pub struct Options {
    /// The vault to run against; `cards` and traversal need one. Names the
    /// workspace as `--workspace` does.
    #[arg(long, global = true, value_name = "DIR", conflicts_with = "workspace")]
    pub vault: Option<PathBuf>,
    /// The workspace to run in, a directory holding `.hmd/` or `hql.toml`.
    /// Without it, the nearest such directory at or above the program's file,
    /// or above the working directory for an expression or standard input.
    #[arg(long, global = true, value_name = "DIR")]
    pub workspace: Option<PathBuf>,
    /// Run with no workspace: no `.hmd/` or `hql.toml` is discovered. User
    /// and system configuration still apply.
    #[arg(long, global = true, conflicts_with_all = ["workspace", "vault"])]
    pub bare: bool,
    /// The named context to run under for this invocation, in place of
    /// `$HQL_CONTEXT` and the configured `current-context`.
    #[arg(long, global = true, value_name = "NAME")]
    pub context: Option<String>,
    /// A module root: a directory whose `.hql` files are the modules a program
    /// imports by their position, `lib/a/b.hql` as `import a.b`. May be
    /// repeated, and adds to the roots the vault's `hql.toml` names.
    #[arg(long, global = true, value_name = "DIR")]
    pub modules: Vec<PathBuf>,
    /// What the answer is wrapped in: the value alone, or one JSON document
    /// carrying the type, the value and the diagnostics. How the value itself
    /// is written is `--to`.
    #[arg(long, global = true, value_enum, default_value_t = Shape::Text)]
    pub envelope: Shape,
    /// The presentation format a result is written in when the program did
    /// not format it: `hql` (the default), `text`, `markdown`, `hmd`, `json`,
    /// `yaml`, `csv`, `d2` or `mermaid`; `none` evaluates and writes no value.
    #[arg(long, global = true, value_name = "FORMAT")]
    pub to: Option<String>,
    /// The format standard input is read in where a program reads `stdin`
    /// and names none: `json`, `jsonlines`, `yaml`, `csv`, `lines` or `text`.
    #[arg(long, global = true, value_name = "FORMAT")]
    pub from: Option<String>,
    /// Write each JSON document of a result on one line, as `jq -c` does; a
    /// stream is then JSON Lines.
    #[arg(long, global = true)]
    pub compact: bool,
    /// Every logger's threshold, overriding the one it was constructed with:
    /// `trace`, `debug`, `info`, `warn` or `error`.
    #[arg(long, global = true, value_name = "LEVEL")]
    pub log_level: Option<String>,
    /// The evaluation instant `time.now()` gives, in RFC 3339:
    /// `2026-10-24T07:30:00Z`. Without it, the host's clock, read once per
    /// evaluation.
    #[arg(long, global = true, value_name = "INSTANT")]
    pub now: Option<String>,
    /// The evaluation zone `time.local_zone()` gives: an IANA name or an
    /// offset `±HH:MM`. Without it, `TZ`, else the host's zone.
    #[arg(long, global = true, value_name = "ZONE")]
    pub zone: Option<String>,
    /// The evaluation seed the unseeded draws of `random` read: an `Int`.
    /// Without it, a seed from the host's entropy, reported at `info`.
    #[arg(long, global = true, value_name = "INT", allow_negative_numbers = true)]
    pub seed: Option<i64>,
    /// A named argument `io.env`'s `arg` reads, `--arg species=owl`; may be
    /// repeated, each name once. A name is `[a-z_][a-z0-9_]*`.
    #[arg(
        long = "arg",
        global = true,
        value_name = "NAME=VALUE",
        value_parser = crate::extensions::env::argument
    )]
    pub arguments: Vec<(String, String)>,
    /// When to stop: `strict` at the first error, `collect` to report them all.
    #[arg(long, global = true, value_name = "MODE")]
    pub report: Option<String>,
    /// The execution backend that runs a checked program: `localhost`, the
    /// in-memory evaluator, unless `hql.toml` or `HQL_BACKEND` names
    /// another. The answer does not depend on it.
    #[arg(long, global = true, value_name = "NAME")]
    pub backend: Option<String>,
    /// The lowest severity shown: `debug`, `info`, `warning` or `error`.
    /// `--envelope json` carries every entry whatever this says.
    #[arg(long, global = true, value_name = "LEVEL")]
    pub report_level: Option<String>,
    /// The format diagnostics are written to standard error in: `text`, a
    /// heading, the source line and a caret, or `json`, one document per
    /// diagnostic per line.
    #[arg(long, global = true, value_enum, default_value_t = DiagnosticsFormat::Text)]
    pub diagnostics_format: DiagnosticsFormat,
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
    /// Print the plan of a program from a file without evaluating it: each
    /// pipeline's steps, with the stage the planner gives each on the
    /// backend in force.
    Plan {
        /// A file, or `-` for standard input.
        file: String,
    },
    /// Print a configured endpoint's effective boundary: for each field of
    /// the type it is read as, its external field, both types, its position,
    /// whether it is read and written, what reads as absent, and the layer
    /// each property comes from.
    Boundary {
        /// A program declaring the type the source is read as, or `-` for
        /// standard input.
        file: String,
        /// The endpoint's id, `[endpoint.<id>]` in the vault's hql.toml.
        id: String,
        /// The type the source is read as; the source's `element` without it.
        #[arg(long = "as", value_name = "TYPE")]
        element: Option<String>,
    },
    /// Type-check and evaluate a program from a file.
    Run {
        /// A file, or `-` for standard input.
        file: String,
    },
    /// Read, evaluate and print, one program at a time.
    Repl,
    /// Rewrite the reference pages' marked parts from the library's doc
    /// strings, creating the page of a primitive that has none.
    #[command(hide = true)]
    Reference {
        /// The site's reference folder, `doc/public/reference`.
        dir: PathBuf,
        /// Write nothing; fail when a page differs from the library.
        #[arg(long)]
        check: bool,
    },
    /// Run the HQL cells in a document and write each one's output.
    Render {
        /// A Markdown or HyperMarkDown document, or `-` for standard input.
        /// Several documents need `--in-place`, and are rendered in turn.
        #[arg(value_name = "FILE", required = true, num_args = 1..)]
        files: Vec<String>,
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
    /// Print the declarations of a name, each with its module and its doc
    /// string; with no name, this help.
    Help {
        /// A function, type, trait or module, bare or qualified: `range`,
        /// `seq.range`, `shelf`.
        name: Option<String>,
        /// A program whose own declarations are searched first.
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Show the reporting settings in force, and where each came from; or
    /// list, show and select named contexts.
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Show the execution context: the mode, the workspace and vault, the
    /// anchor discovery started from, and where each came from.
    Context {
        /// A program file, whose directory is the anchor; the working
        /// directory without one.
        file: Option<String>,
    },
    /// Check the semantic-search chain — vault, configuration, index, model,
    /// query set — and name the first part that is missing.
    Doctor,
}

/// What `hql config` does with the named contexts of the system and user
/// configuration.
#[derive(Debug, Subcommand)]
enum ConfigAction {
    /// Print the persistent `current-context`.
    CurrentContext,
    /// List the named contexts, marking the current one.
    GetContexts,
    /// Set `current-context` in the user configuration file.
    UseContext {
        /// A defined context.
        name: String,
    },
    /// Print the system and user configuration, merged.
    View,
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

/// `hql reference`: rewrite the reference pages, or with `--check` name
/// each that differs from the library.
fn reference(
    dir: &Path,
    check: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<u8, String> {
    if check {
        let problems = crate::reference::check(dir);
        for problem in &problems {
            writeln!(err, "{problem}").map_err(|error| error.to_string())?;
        }
        return Ok(u8::from(!problems.is_empty()));
    }
    let report = crate::reference::write(dir)?;
    for (label, paths) in [
        ("updated", &report.updated),
        ("created", &report.created),
        ("names no primitive", &report.orphaned),
    ] {
        for path in paths {
            writeln!(out, "{label}: {}", dir.join(path).display())
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(0)
}

/// `hql help`: a name's declarations and doc strings; with no name, the
/// usage; and for a name nothing declares that names a command, that
/// command's help.
fn help(
    name: Option<&str>,
    file: Option<&Path>,
    vault: &Vault,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<u8, String> {
    use clap::CommandFactory;
    let Some(name) = name else {
        write!(out, "{}", Cli::command().render_long_help()).map_err(|error| error.to_string())?;
        return Ok(0);
    };
    let program = match file {
        Some(path) => Some(crate::help::Module::program(
            &fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?,
        )),
        None => None,
    };
    let modules = crate::help::scope(program, &vault.extensions.module_roots);
    match crate::help::help(name, &modules) {
        Ok(text) => {
            write!(out, "{text}").map_err(|error| error.to_string())?;
            Ok(0)
        }
        Err(message) => {
            let mut command = Cli::command();
            if let Some(subcommand) = command.find_subcommand_mut(name) {
                let mut subcommand = subcommand.clone().bin_name(format!("hql {name}"));
                write!(out, "{}", subcommand.render_long_help())
                    .map_err(|error| error.to_string())?;
                return Ok(0);
            }
            writeln!(err, "{message}").map_err(|error| error.to_string())?;
            Ok(1)
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
const COMMANDS: [&str; 13] = [
    "context",
    "eval",
    "check",
    "plan",
    "boundary",
    "run",
    "repl",
    "render",
    "builtins",
    "config",
    "doctor",
    "help",
    "reference",
];

/// Options that take a value, so the word after one is not the program.
const VALUED: [&str; 16] = [
    "--context",
    "--vault",
    "--workspace",
    "--now",
    "--zone",
    "--seed",
    "--arg",
    "--envelope",
    "--to",
    "--from",
    "--report",
    "--report-level",
    "--log-level",
    "--modules",
    "--backend",
    "--diagnostics-format",
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

/// What `hql --version` writes after `hql`: the crate's version, and on a
/// line of its own the time-zone database zone names resolve against, since
/// a future instant read in a zone can change with it.
#[must_use]
pub fn version() -> String {
    format!(
        "{}\ntime-zone database: {}",
        env!("CARGO_PKG_VERSION"),
        crate::time::ZoneDatabase::in_use()
    )
}

/// Parse arguments from the process, for the binary.
///
/// A combination clap cannot express is still a usage error, exit `2`.
#[must_use]
pub fn parse() -> Cli {
    use clap::{CommandFactory, FromArgMatches};
    static VERSION: std::sync::LazyLock<String> = std::sync::LazyLock::new(version);
    let matches = Cli::command()
        .version(VERSION.as_str())
        .get_matches_from(with_default_command(std::env::args().collect()));
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    if let Command::Render {
        files,
        in_place,
        emit,
        cell,
        ..
    } = &cli.command
    {
        let refused = if files.len() > 1 && !*in_place {
            Some("several files need --in-place: each is written over itself")
        } else if files.len() > 1 && cell.is_some() {
            Some("--cell names a cell of one document, not of several")
        } else if files.len() > 1 && files.iter().any(|file| file == "-") {
            Some("standard input is rendered alone, not beside other files")
        } else if *emit != Emit::Replace {
            None
        } else if *in_place {
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
    // An argument named twice is refused before anything is evaluated.
    if let Some(name) = crate::extensions::env::repeated(&cli.options.arguments) {
        use clap::CommandFactory;
        Cli::command()
            .error(
                clap::error::ErrorKind::ArgumentConflict,
                format!("--arg {name} is given twice: each argument is named once"),
            )
            .exit();
    }
    cli
}

fn execute(cli: &Cli, out: &mut impl Write, err: &mut impl Write) -> Result<u8, String> {
    if let Some(level) = &cli.options.log_level
        && crate::extensions::logging::Level::parse(level).is_none()
    {
        return Err(format!(
            "unknown log level `{level}`; expected `trace`, `debug`, `info`, `warn` or `error`"
        ));
    }
    crate::io::set_log_level(cli.options.log_level.clone());
    let now = match cli.options.now.as_deref() {
        None => None,
        Some(text) => Some(crate::time::parse_instant(text).ok_or_else(|| {
            format!("--now {text:?} is not an RFC 3339 instant such as 2026-10-24T07:30:00Z")
        })?),
    };
    if let Some(zone) = &cli.options.zone {
        crate::time::zone(zone).map_err(|message| format!("--zone: {message}"))?;
    }
    crate::time::set_clock(crate::time::Clock {
        now,
        zone: cli.options.zone.clone(),
    });
    crate::random::set_seeding(crate::random::Seeding {
        seed: cli.options.seed,
        report: true,
    });
    crate::io::set_stdin_format(match cli.options.from.as_deref() {
        None => None,
        Some(name) => {
            let format = crate::extensions::format::format_named(name)
                .ok_or_else(|| format!("unknown format `{name}` for --from"))?;
            crate::io::document_type(format).map_err(|_| {
                format!(
                    "--from names a format standard input is read in; {name} is written, not read"
                )
            })?;
            Some(format)
        }
    });
    let environment = crate::context::Environment::from_process()?;
    // The context commands read the configuration alone, so they work while
    // the selected context does not resolve: that is when one is fixed.
    if let Command::Config {
        action: Some(action),
    } = &cli.command
    {
        return config_action(action, &environment, out, err);
    }
    let context = crate::context::ExecutionContext::construct(&request(cli), &environment)
        .map_err(|error| error.to_string())?;
    // What `io.env` reads: the arguments, and the variables the workspace
    // grants; `hql render` grants none, by its host policy.
    let grant = context
        .env_grant()
        .map_err(|error| error.to_string())?
        .map_or(
            crate::extensions::env::Grant::All,
            crate::extensions::env::Grant::Only,
        );
    crate::extensions::env::set_invocation(grant, cli.options.arguments.clone());
    // The run's diagnostic channel exists before the loader runs, so the
    // loader writes into it as every later phase does.
    let opened = crate::io::open_diagnostics();
    let result = context.load_workspace().and_then(|vault| {
        for diagnostic in &vault.warnings {
            crate::io::write_diagnostic(diagnostic.clone());
        }
        dispatch(cli, &context, &vault, out, err)
    });
    // Every diagnostic was delivered before the run ends: the channel
    // closes, and only then is the exit status returned.
    if opened {
        crate::io::close_diagnostics();
    }
    result
}

/// What the command line states about the execution context: the anchor of
/// the program file a command reads, and the workspace options.
fn request(cli: &Cli) -> crate::context::Request {
    let file = match &cli.command {
        Command::Run { file }
        | Command::Check { file }
        | Command::Plan { file }
        | Command::Boundary { file, .. } => Some(file.as_str()),
        Command::Context { file } => file.as_deref(),
        _ => None,
    };
    let workspace = match (&cli.options.workspace, &cli.options.vault) {
        (Some(dir), _) => Some((dir.clone(), "--workspace")),
        (None, Some(dir)) => Some((dir.clone(), "--vault")),
        (None, None) => None,
    };
    crate::context::Request {
        anchor: file.and_then(crate::context::Request::anchor_of),
        bare: cli.options.bare,
        workspace,
        modules: cli.options.modules.clone(),
        context: cli.options.context.clone(),
        ..crate::context::Request::default()
    }
}

/// `hql config <action>`: the named contexts of the system and user
/// configuration.
fn config_action(
    action: &ConfigAction,
    environment: &crate::context::Environment,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<u8, String> {
    let layers = crate::context::Layers::load(environment).map_err(|error| error.to_string())?;
    let current = layers
        .current_context()
        .map_err(|error| error.to_string())?;
    match action {
        ConfigAction::CurrentContext => {
            let Some(current) = current else {
                writeln!(err, "hql: no current-context is set")
                    .map_err(|error| error.to_string())?;
                return Ok(1);
            };
            writeln!(out, "{}", current.value).map_err(|error| error.to_string())?;
            Ok(0)
        }
        ConfigAction::GetContexts => {
            let contexts = layers.contexts().map_err(|error| error.to_string())?;
            let mut rows = vec![[
                "CURRENT".to_owned(),
                "NAME".to_owned(),
                "WORKSPACE".to_owned(),
                "DEFINED IN".to_owned(),
            ]];
            for (name, named) in &contexts {
                let marked = current
                    .as_ref()
                    .is_some_and(|current| &current.value == name);
                let workspace = named.workspace().map_err(|error| error.to_string())?;
                rows.push([
                    if marked { "*" } else { "" }.to_owned(),
                    name.clone(),
                    workspace.map_or_else(|| "-".to_owned(), |root| root.display().to_string()),
                    named.path.display().to_string(),
                ]);
            }
            let width = |column: usize| rows.iter().map(|row| row[column].len()).max().unwrap_or(0);
            let widths = [width(0), width(1), width(2)];
            for row in &rows {
                writeln!(
                    out,
                    "{:<w0$}  {:<w1$}  {:<w2$}  {}",
                    row[0],
                    row[1],
                    row[2],
                    row[3],
                    w0 = widths[0],
                    w1 = widths[1],
                    w2 = widths[2]
                )
                .map_err(|error| error.to_string())?;
            }
            Ok(0)
        }
        ConfigAction::UseContext { name } => {
            let contexts = layers.contexts().map_err(|error| error.to_string())?;
            if !contexts.contains_key(name) {
                return Err(crate::context::ContextError::UnknownContext {
                    name: name.clone(),
                    selected_by: "use-context".to_owned(),
                    known: contexts.into_keys().collect(),
                }
                .to_string());
            }
            let home = layers
                .home
                .as_ref()
                .ok_or("no HQL home is known: set $HQL_HOME or $HOME")?;
            let path = home.config_file();
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
                Err(error) => return Err(format!("{}: {error}", path.display())),
            };
            fs::create_dir_all(&home.path)
                .map_err(|error| format!("{}: {error}", home.path.display()))?;
            fs::write(&path, crate::context::set_current_context(&text, name))
                .map_err(|error| format!("{}: {error}", path.display()))?;
            writeln!(out, "switched to context `{name}`").map_err(|error| error.to_string())?;
            Ok(0)
        }
        ConfigAction::View => {
            let text = toml::to_string(&layers.merged()).map_err(|error| error.to_string())?;
            write!(out, "{text}").map_err(|error| error.to_string())?;
            Ok(0)
        }
    }
}

/// A setting the context's configuration layers state, below the workspace
/// and the environment.
fn configured(
    context: &crate::context::ExecutionContext,
    section: &str,
    key: &str,
) -> Result<Option<crate::context::Contributed<String>>, String> {
    context
        .setting(section, key)
        .map_err(|error| error.to_string())
}

/// `hql context`: the execution context, each part with where it came from.
fn write_context(
    context: &crate::context::ExecutionContext,
    options: &Options,
    out: &mut impl Write,
) -> Result<(), String> {
    let path = |path: Option<&Path>| path.map(|path| path.display().to_string());
    let workspace = context.workspace.as_ref();
    if options.envelope == Shape::Json {
        let document = serde_json::json!({
            "mode": context.mode().label(),
            "context": context.context.as_ref().map(|choice| serde_json::json!({
                "name": choice.value,
                "origin": choice.origin.label(),
            })),
            "workspace": path(context.workspace_root()),
            "workspace_origin": workspace.map(|workspace| workspace.origin.label()),
            "vault": path(context.vault()),
            "anchor": context.anchor.display().to_string(),
            "cwd": context.cwd.display().to_string(),
            "hql_home": context.layers.home.as_ref().map(|home| serde_json::json!({
                "path": home.path.display().to_string(),
                "origin": home.origin.label(),
            })),
            "config": context.config_files().map(|file| serde_json::json!({
                "layer": file.origin.label(),
                "path": file.path.display().to_string(),
                "present": file.is_present(),
            })).collect::<Vec<_>>(),
            "module_roots": context.module_roots.iter().map(|root| serde_json::json!({
                "path": root.value.display().to_string(),
                "origin": root.origin.label(),
            })).collect::<Vec<_>>(),
            "imports": context.imports.iter().map(|import| serde_json::json!({
                "name": import.value,
                "origin": import.origin.label(),
            })).collect::<Vec<_>>(),
            "prelude": { "value": context.prelude.value, "origin": context.prelude.origin.label() },
        });
        let text = if options.compact {
            serde_json::to_string(&document)
        } else {
            serde_json::to_string_pretty(&document)
        }
        .map_err(|error| error.to_string())?;
        return writeln!(out, "{text}").map_err(|error| error.to_string());
    }
    let none = || "none".to_owned();
    let lines = [
        ("mode", context.mode().label().to_owned()),
        (
            "context",
            context.context.as_ref().map_or_else(none, |choice| {
                format!("{} ({})", choice.value, choice.origin.label())
            }),
        ),
        (
            "workspace",
            workspace.map_or_else(none, |workspace| {
                format!(
                    "{} ({})",
                    workspace.root.display(),
                    workspace.origin.label()
                )
            }),
        ),
        ("vault", path(context.vault()).unwrap_or_else(none)),
        ("anchor", context.anchor.display().to_string()),
        ("cwd", context.cwd.display().to_string()),
        (
            "hql-home",
            context.layers.home.as_ref().map_or_else(none, |home| {
                format!("{} ({})", home.path.display(), home.origin.label())
            }),
        ),
        (
            "prelude",
            format!(
                "{} ({})",
                if context.prelude.value { "on" } else { "off" },
                context.prelude.origin.label()
            ),
        ),
    ];
    for (label, value) in lines {
        writeln!(out, "{:<11}{value}", format!("{label}:")).map_err(|error| error.to_string())?;
    }
    let mut list = |label: &str, rows: Vec<(String, String)>| -> Result<(), String> {
        writeln!(out, "{label}:").map_err(|error| error.to_string())?;
        if rows.is_empty() {
            writeln!(out, "  none").map_err(|error| error.to_string())?;
        }
        for (value, origin) in rows {
            writeln!(out, "  {value} ({origin})").map_err(|error| error.to_string())?;
        }
        Ok(())
    };
    list(
        "config",
        context
            .config_files()
            .map(|file| {
                let state = if file.is_present() { "" } else { ", absent" };
                (
                    file.path.display().to_string(),
                    format!("{}{state}", file.origin.label()),
                )
            })
            .collect(),
    )?;
    list(
        "module roots",
        context
            .module_roots
            .iter()
            .map(|root| (root.value.display().to_string(), root.origin.label()))
            .collect(),
    )?;
    list(
        "imports",
        context
            .imports
            .iter()
            .map(|import| (import.value.clone(), import.origin.label()))
            .collect(),
    )?;
    Ok(())
}

fn dispatch(
    cli: &Cli,
    context: &crate::context::ExecutionContext,
    vault: &Vault,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<u8, String> {
    let root = context.workspace_root();
    let requested = match &cli.options.report {
        Some(name) => Some(Mode::parse(name).ok_or_else(|| {
            format!("unknown report mode `{name}`; expected `strict` or `collect`")
        })?),
        None => None,
    };
    let mut settings = Settings::resolve(requested, root);
    if settings.is_default()
        && let Some(stated) = configured(context, "report", "mode")?
    {
        settings = Settings {
            mode: Mode::parse(&stated.value).ok_or_else(|| {
                format!(
                    "unknown report mode `{}`, which {} states; expected `strict` or `collect`",
                    stated.value,
                    stated.origin.label()
                )
            })?,
            origin: stated.origin.label(),
        };
    }
    let level = match &cli.options.report_level {
        Some(name) => Some(Severity::parse_level(name).ok_or_else(|| {
            format!("unknown report level `{name}`; expected `debug`, `info`, `warning` or `error`")
        })?),
        None => None,
    };
    let mut level = Level::resolve(level, root);
    if level.is_default()
        && let Some(stated) = configured(context, "report", "level")?
    {
        level = Level {
            severity: Severity::parse_level(&stated.value).ok_or_else(|| {
                format!(
                    "unknown report level `{}`, which {} states; expected `debug`, `info`, \
                     `warning` or `error`",
                    stated.value,
                    stated.origin.label()
                )
            })?,
            origin: stated.origin.label(),
        };
    }
    let shown = level.severity;
    // An unregistered backend fails every program, and `hql config`, naming
    // the layer that supplied it; listing the builtins needs none.
    let backend = match crate::backends::Selection::resolve(cli.options.backend.as_deref(), root) {
        Ok(selection) if selection.origin == crate::backends::Origin::Default => {
            match configured(context, "execution", "backend")? {
                Some(stated) => {
                    let origin = match stated.origin {
                        crate::context::Origin::System => crate::backends::Origin::System,
                        crate::context::Origin::User => crate::backends::Origin::User,
                        _ => crate::backends::Origin::Context,
                    };
                    crate::backends::Selection::of(&stated.value, origin)
                }
                None => Ok(selection),
            }
        }
        resolved => resolved,
    };
    if !matches!(
        cli.command,
        Command::Builtins | Command::Help { .. } | Command::Reference { .. }
    ) && let Err(refused) = &backend
    {
        return Err(refused.clone());
    }
    let backend = backend.unwrap_or_else(|_| crate::backends::Selection::standard());

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
                        "  {}\n    {}\n    {}, {}\n    {}",
                        step.name,
                        step.signature.replace('\n', "\n    "),
                        step.purity.label(),
                        step.shape.label(),
                        step.description
                    )
                    .map_err(|error| error.to_string())?;
                }
            }
            // The combiners a fold may name in its `by` position, with the
            // laws a planner reads to split it.
            writeln!(out, "combiners").map_err(|error| error.to_string())?;
            for combiner in crate::extensions::COMBINERS {
                let laws: Vec<&str> = [
                    (combiner.associative, "associative"),
                    (combiner.commutative, "commutative"),
                    (!combiner.commutative, "order-sensitive"),
                ]
                .into_iter()
                .filter_map(|(holds, law)| holds.then_some(law))
                .collect();
                writeln!(
                    out,
                    "  {}\n    over {}, identity {}\n    {}; folds {}",
                    combiner.name,
                    combiner.over,
                    combiner.identity.unwrap_or("none"),
                    laws.join(", "),
                    combiner.input()
                )
                .map_err(|error| error.to_string())?;
            }
            Ok(0)
        }
        Command::Doctor => {
            let checks = crate::extensions::semantic::doctor(vault);
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
        Command::Config { .. } => {
            writeln!(
                out,
                "report mode: {} (from {})\nreport level: {} (from {})\nbackend: {} (from {})\nvault: {}",
                settings.mode.label(),
                settings.origin,
                level.severity.label(),
                level.origin,
                backend.backend.name,
                backend.origin.label(),
                if vault.is_present() {
                    format!("{} ({} cards)", vault.root.display(), vault.cards.len())
                } else {
                    "none".to_owned()
                }
            )
            .map_err(|error| error.to_string())?;
            Ok(0)
        }
        Command::Context { .. } => {
            write_context(context, &cli.options, out)?;
            Ok(0)
        }
        Command::Eval { expression } => {
            crate::io::set_host(crate::io::Policy::Open, PathBuf::from("."));
            let outcome = crate::run_on(expression, vault, settings.mode, backend);
            emit(
                &outcome,
                expression,
                "<expression>",
                HQL,
                &cli.options,
                vault,
                shown,
                out,
                err,
            )
        }
        Command::Run { file } => {
            let (source, origin) = read(file)?;
            crate::io::set_host(crate::io::Policy::Open, directory_of(file));
            if file == "-" {
                crate::io::set_program_on_stdin();
            }
            let outcome = crate::run_on(&source, vault, settings.mode, backend);
            emit(
                &outcome,
                &source,
                &origin,
                HQL,
                &cli.options,
                vault,
                shown,
                out,
                err,
            )
        }
        Command::Check { file } => {
            let (source, origin) = read(file)?;
            let outcome = crate::check_reported(&source, vault, settings.mode);
            emit(
                &outcome,
                &source,
                &origin,
                TEXT,
                &cli.options,
                vault,
                shown,
                out,
                err,
            )
        }
        Command::Plan { file } => {
            let (source, origin) = read(file)?;
            let Ok(planned) = crate::plan(&source, vault, backend.backend) else {
                // What stopped it is what checking reports.
                let outcome = crate::check_reported(&source, vault, settings.mode);
                return emit(
                    &outcome,
                    &source,
                    &origin,
                    TEXT,
                    &cli.options,
                    vault,
                    shown,
                    out,
                    err,
                );
            };
            write_plan(&planned, &source, backend.backend.name, &cli.options, out)?;
            Ok(0)
        }
        Command::Boundary { file, id, element } => {
            let (source, _) = read(file)?;
            match crate::inspect_boundary(&source, vault, id, element.as_deref()) {
                Ok(inspected) => {
                    write_boundary(&inspected, &cli.options, out)?;
                    Ok(0)
                }
                Err(message) => {
                    writeln!(err, "error: {message}").map_err(|error| error.to_string())?;
                    Ok(1)
                }
            }
        }
        Command::Render {
            files,
            write,
            in_place,
            emit,
            cell,
        } => {
            if files.len() > 1 && cli.options.envelope == Shape::Json {
                return Err("--envelope json renders one document, not several".to_owned());
            }
            let mut render = |file: &String| -> Result<u8, String> {
                let (source, origin) = read(file)?;
                // A document is a card of a vault: the one `--vault` names,
                // or the nearest at or above the document. In none, no cell
                // runs.
                let found;
                let vault = match document_vault(file, cli, context, vault)? {
                    Some(Found::Loaded) => vault,
                    Some(Found::Own(own)) => {
                        let own = *own;
                        for diagnostic in &own.warnings {
                            crate::io::write_diagnostic(diagnostic.clone());
                        }
                        found = own;
                        &found
                    }
                    None => {
                        crate::io::write_diagnostic(missing_vault(&origin));
                        let written = crate::io::diagnostics_written();
                        deliver(&written, &source, &origin, shown, &cli.options, err)?;
                        return Ok(1);
                    }
                };
                // A document reads beside itself and writes nothing outside.
                let root = directory_of(file);
                crate::io::set_host(crate::io::Policy::Document { root: root.clone() }, root);
                let rendered = transclude::render_cell(
                    &source,
                    vault,
                    settings.mode,
                    (*emit).into(),
                    cell.as_deref(),
                )?;
                // Each cell's diagnostics go into the run's channel, after
                // what the loader wrote.
                for diagnostic in originated(&rendered.diagnostics, &origin) {
                    crate::io::write_diagnostic(diagnostic);
                }
                let written = crate::io::diagnostics_written();
                let failed = crate::diagnostics::reaches(&rendered.diagnostics, Severity::Error);
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
                                "diagnostics": originated(&cell.diagnostics, &origin)
                                    .iter()
                                    .map(Diagnostic::to_json)
                                    .collect::<Vec<_>>(),
                            })
                        })
                        .collect();
                    let document = serde_json::json!({
                        "document": rendered.document,
                        "cells": cells,
                        "diagnostics": written.iter().map(Diagnostic::to_json).collect::<Vec<_>>(),
                    });
                    writeln!(
                        out,
                        "{}",
                        serde_json::to_string_pretty(&document)
                            .unwrap_or_else(|_| "null".to_owned())
                    )
                    .map_err(|error| error.to_string())?;
                    return Ok(u8::from(failed || !rendered.complete));
                }
                deliver(&written, &source, &origin, shown, &cli.options, err)?;
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
                    None => {
                        write!(out, "{}", rendered.document).map_err(|error| error.to_string())?
                    }
                }
                Ok(u8::from(failed))
            };
            // Every document is rendered, and the exit code is the worst of them.
            let mut code = 0;
            for file in files {
                code = code.max(render(file)?);
            }
            Ok(code)
        }
        Command::Reference { dir, check } => reference(dir, *check, out, err),
        Command::Help { name, file } => help(name.as_deref(), file.as_deref(), vault, out, err),
        Command::Repl => repl(
            &repl_vault(vault, shown, &cli.options, err)?,
            settings.mode,
            backend,
            shown,
            &cli.options,
            out,
            err,
        ),
    }
}

/// Write a program's plan: per pipeline, the line it is written on and the
/// expression its first step reads, then one line per step with its
/// operation, shape, purity and stage annotation. `--envelope json` writes
/// the same as one JSON document.
/// A source's effective boundary as `hql boundary` prints it: a line for
/// the source, then each field with each property's value and origin.
fn write_boundary(
    inspected: &crate::InspectedBoundary,
    options: &Options,
    out: &mut impl Write,
) -> Result<(), String> {
    use crate::boundary::Held;
    let boundary = &inspected.boundary;
    let absent = |held: &Held<Vec<crate::endpoints::Absence>>| {
        held.value
            .iter()
            .map(|absence| absence.name())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if options.envelope == Shape::Json {
        let held = |value: serde_json::Value, origin: crate::boundary::Origin| serde_json::json!({ "value": value, "origin": origin.label() });
        let fields: Vec<_> = boundary
            .fields
            .iter()
            .map(|field| {
                serde_json::json!({
                    "field": field.field.name,
                    "type": field.field.ty.to_string(),
                    "optional": field.field.optional,
                    "external": held(
                        crate::boundary::written(&field.external.value).into(),
                        field.external.origin
                    ),
                    "external_type": held(
                        field.external_type.value.clone().into(),
                        field.external_type.origin
                    ),
                    "position": held(field.position.value.into(), field.position.origin),
                    "read": held(field.readable.value.into(), field.readable.origin),
                    "write": held(field.writable.value.into(), field.writable.origin),
                    "absent": held(absent(&field.absent).into(), field.absent.origin),
                    "lossy": held(field.lossy.value.into(), field.lossy.origin),
                })
            })
            .collect();
        let document = serde_json::json!({
            "source": boundary.source,
            "kind": boundary.kind,
            "element": inspected.element,
            "strictness": boundary.strictness.name(),
            "mode": inspected.mode.name(),
            "fields": fields,
        });
        let text = if options.compact {
            serde_json::to_string(&document)
        } else {
            serde_json::to_string_pretty(&document)
        }
        .map_err(|error| error.to_string())?;
        return writeln!(out, "{text}").map_err(|error| error.to_string());
    }
    writeln!(
        out,
        "endpoint {} ({}), read as {}; strictness {}, mode {}",
        boundary.source,
        boundary.kind,
        inspected.element,
        boundary.strictness.name(),
        inspected.mode.name()
    )
    .map_err(|error| error.to_string())?;
    for field in &boundary.fields {
        let optional = if field.field.optional { "?" } else { "" };
        writeln!(out, "{}{optional} : {}", field.field.name, field.field.ty)
            .map_err(|error| error.to_string())?;
        let rows = [
            (
                "external",
                crate::boundary::written(&field.external.value),
                field.external.origin,
            ),
            (
                "type",
                field.external_type.value.clone(),
                field.external_type.origin,
            ),
            (
                "position",
                field.position.value.to_string(),
                field.position.origin,
            ),
            (
                "read",
                field.readable.value.to_string(),
                field.readable.origin,
            ),
            (
                "write",
                field.writable.value.to_string(),
                field.writable.origin,
            ),
            ("absent", absent(&field.absent), field.absent.origin),
        ];
        let width = rows
            .iter()
            .map(|(_, value, _)| value.len())
            .max()
            .unwrap_or(0);
        for (property, value, origin) in rows {
            writeln!(out, "  {property:<9}{value:<width$}  {}", origin.label())
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn write_plan(
    planned: &[crate::PlannedPipeline],
    source: &str,
    backend: &str,
    options: &Options,
    out: &mut impl Write,
) -> Result<(), String> {
    let line_of = |at: usize| source[..at.min(source.len())].matches('\n').count() + 1;
    if options.envelope == Shape::Json {
        let pipelines: Vec<_> = planned
            .iter()
            .map(|pipeline| {
                serde_json::json!({
                    "line": line_of(pipeline.span.start),
                    "span": [pipeline.span.start, pipeline.span.end],
                    "input": pipeline.input,
                    "steps": steps_json(&pipeline.steps),
                })
            })
            .collect();
        let document = serde_json::json!({ "backend": backend, "pipelines": pipelines });
        let text = if options.compact {
            serde_json::to_string(&document)
        } else {
            serde_json::to_string_pretty(&document)
        }
        .map_err(|error| error.to_string())?;
        return writeln!(out, "{text}").map_err(|error| error.to_string());
    }
    writeln!(out, "backend {backend}").map_err(|error| error.to_string())?;
    for pipeline in planned {
        writeln!(
            out,
            "line {}: {}",
            line_of(pipeline.span.start),
            pipeline
                .input
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        )
        .map_err(|error| error.to_string())?;
        write_steps(out, &pipeline.steps, "  ")?;
    }
    Ok(())
}

/// Planned steps as `hql plan --to json` writes them, a fan-out's branches
/// within the step.
fn steps_json(steps: &[crate::PlannedStep]) -> Vec<serde_json::Value> {
    steps
        .iter()
        .map(|step| {
            let mut written = serde_json::json!({
                "name": step.name,
                "shape": step.shape.label(),
                "purity": step.purity.label(),
                "stage": step.stage.index,
                "strategy": step.stage.strategy.label(),
            });
            if let Some(topology) = step.topology {
                written["topology"] = serde_json::json!(topology);
            }
            if !step.branches.is_empty() {
                written["branches"] = step
                    .branches
                    .iter()
                    .map(|branch| {
                        serde_json::json!({ "name": branch.name, "steps": steps_json(&branch.steps) })
                    })
                    .collect();
            }
            written
        })
        .collect()
}

/// Planned steps, one per line under `indent`, each fan-out's branches
/// beneath it as the stages that read the step's input.
fn write_steps(
    out: &mut dyn Write,
    steps: &[crate::PlannedStep],
    indent: &str,
) -> Result<(), String> {
    let width = steps.iter().map(|step| step.name.len()).max().unwrap_or(0);
    for step in steps {
        let topology = step
            .topology
            .map(|topology| format!(", {topology}"))
            .unwrap_or_default();
        writeln!(
            out,
            "{indent}| {:<width$}  stage {}  {:<12}  {}, {}{topology}",
            step.name,
            step.stage.index,
            step.stage.strategy.label(),
            step.shape.label(),
            step.purity.label(),
        )
        .map_err(|error| error.to_string())?;
        for branch in &step.branches {
            writeln!(out, "{indent}    {} =", branch.name).map_err(|error| error.to_string())?;
            write_steps(out, &branch.steps, &format!("{indent}      "))?;
        }
    }
    Ok(())
}

/// The vault a document renders in, besides the one the run loaded.
enum Found {
    /// The vault the run already loaded.
    Loaded,
    /// The document's own vault, found from it and loaded for it.
    Own(Box<Vault>),
}

/// The vault of the document `file`: the workspace an explicit selection
/// names, or the nearest directory at or above the document holding `.hmd/`
/// or `hql.toml`; standard input's is the run's. `None` where there is none,
/// and always under `--bare`: the document is a card of no vault.
fn document_vault(
    file: &str,
    cli: &Cli,
    context: &crate::context::ExecutionContext,
    loaded: &Vault,
) -> Result<Option<Found>, String> {
    let named = context
        .workspace
        .as_ref()
        .is_some_and(|workspace| workspace.origin.is_explicit());
    if named || file == "-" {
        return Ok(loaded.is_present().then_some(Found::Loaded));
    }
    if context.bare {
        return Ok(None);
    }
    let loaded_root = context.workspace_root();
    let document = fs::canonicalize(file).map_err(|error| format!("{file}: {error}"))?;
    let Some(found) = document.parent().and_then(vault::discover) else {
        return Ok(None);
    };
    let same = loaded_root
        .and_then(|root| fs::canonicalize(root).ok())
        .is_some_and(|root| root == found);
    if same {
        return Ok(Some(Found::Loaded));
    }
    // The document's own workspace, under the same system and user layers
    // as the run's.
    let request = crate::context::Request {
        anchor: Some(found),
        ..request(cli)
    };
    let own = crate::context::ExecutionContext::construct(&request, &context.environment)
        .map_err(|error| error.to_string())?;
    Ok(Some(Found::Own(Box::new(own.load_workspace()?))))
}

/// The load failure of a document in no vault.
fn missing_vault(origin: &str) -> Diagnostic {
    let mut refused = Diagnostic::new(
        crate::diagnostics::Phase::Load,
        crate::diagnostics::Kind::MissingVault,
        Severity::Failure,
        crate::diagnostics::Disposition::Fatal,
        0..0,
        "no directory at or above this document holds `.hmd/` or `hql.toml`, so it is a \
         card of no vault",
    )
    .with_help("render it inside a vault, or name one with `--vault <dir>`");
    refused.origin = Some(origin.to_owned());
    refused
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
        deliver(&outcome.diagnostics, source, origin, shown, options, err)?;
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
                "diagnostics": originated(&outcome.diagnostics, origin)
                    .iter()
                    .map(Diagnostic::to_json)
                    .collect::<Vec<_>>(),
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
                for signature in outcome.diagnostics.iter().filter(|diagnostic| {
                    matches!(
                        diagnostic.kind,
                        crate::diagnostics::Kind::Signature | crate::diagnostics::Kind::Effect
                    )
                }) {
                    writeln!(out, "{}", signature.message).map_err(|error| error.to_string())?;
                }
                // `check` has a type and no value; that is the whole answer.
                writeln!(out, "{inferred}").map_err(|error| error.to_string())?;
            }
            deliver(&outcome.diagnostics, source, origin, shown, options, err)?;
        }
    }
    Ok(u8::from(outcome.failed()))
}

/// The diagnostics with `origin` where they name none: what the program
/// was read from. A cell's diagnostic names only the cell, `#id`, and the
/// document's path goes before it.
fn originated(diagnostics: &[Diagnostic], origin: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            diagnostic.origin = Some(match diagnostic.origin.take() {
                None => origin.to_owned(),
                Some(cell) if cell.starts_with('#') => format!("{origin}{cell}"),
                Some(other) => other,
            });
            diagnostic
        })
        .collect()
}

/// Deliver the channel's contents to the default target, `stderr`, in the
/// format the host's configuration names: the default wiring
/// `diagnostics | DiagnosticText | stderr`. The shown level filters what
/// is delivered, never what the channel holds.
fn deliver(
    diagnostics: &[Diagnostic],
    source: &str,
    origin: &str,
    shown: Severity,
    options: &Options,
    err: &mut impl Write,
) -> Result<(), String> {
    let shown: Vec<Diagnostic> = originated(diagnostics, origin)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity >= shown)
        .collect();
    match options.diagnostics_format {
        DiagnosticsFormat::Json => {
            for diagnostic in &shown {
                writeln!(err, "{}", diagnostic.to_json()).map_err(|error| error.to_string())?;
            }
        }
        DiagnosticsFormat::Text => {
            for diagnostic in &shown {
                // A diagnostic about another origin has no line of this
                // source to point at.
                let file = diagnostic
                    .origin
                    .as_deref()
                    .map(|written| written.split_once('#').map_or(written, |(file, _)| file));
                // A load diagnostic is about a document as a whole, never a
                // place in the source.
                let text = (file == Some(origin)
                    && diagnostic.phase != crate::diagnostics::Phase::Load)
                    .then_some(source);
                writeln!(
                    err,
                    "{}\n",
                    render::diagnostic_text(diagnostic, text, origin)
                )
                .map_err(|error| error.to_string())?;
            }
            if let Some(summary) = render::summary(&shown) {
                writeln!(err, "{summary}").map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

/// The vault a REPL session runs against. Each line typed is its own run,
/// with its own diagnostic channel; what the loader wrote is delivered once,
/// before the first, and the host's channel is closed.
fn repl_vault(
    vault: &Vault,
    shown: Severity,
    options: &Options,
    err: &mut impl Write,
) -> Result<Vault, String> {
    deliver(&crate::io::close_diagnostics(), "", "", shown, options, err)?;
    let mut session = vault.clone();
    session.warnings.clear();
    Ok(session)
}

/// Read, evaluate and print, keeping bindings across lines.
///
/// Bindings persist by replaying the statements that succeeded, which keeps
/// the session a program rather than a sequence of unrelated runs.
fn repl(
    vault: &Vault,
    mode: Mode,
    backend: crate::backends::Selection,
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
    // Each line typed is its own origin, `<repl:N>`, counted in what was
    // typed and not in the bindings replayed before it.
    let mut typed = 0;
    loop {
        let Some(line) = input.next()? else {
            input.close();
            return Ok(0);
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        typed += 1;
        match line {
            // The bare words are what a REPL is commonly left with.
            ":quit" | ":q" | "quit" | "exit" => {
                input.close();
                return Ok(0);
            }
            ":help" | ":h" => {
                writeln!(
                    out,
                    ":type <expression>   the type, without evaluating\n\
                     :type <function>     its declarations, and their doc strings' first sentences\n\
                     :help <name>         the declarations of a name, and their doc strings\n\
                     :vault               what the session is running against\n\
                     :reset               forget the bindings made so far\n\
                     :quit, quit, exit    leave"
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

        if let Some(name) = line.strip_prefix(":help ") {
            let modules = crate::help::scope(
                (!prelude.is_empty()).then(|| crate::help::Module::program(&prelude.join("\n\n"))),
                &vault.extensions.module_roots,
            );
            match crate::help::help(name.trim(), &modules) {
                Ok(text) => write!(out, "{text}").map_err(|error| error.to_string())?,
                Err(message) => writeln!(err, "{message}").map_err(|error| error.to_string())?,
            }
            continue;
        }
        // A declared function's type is its declarations: each form, with
        // the first sentence of its doc string.
        if let Some(name) = line.strip_prefix(":type ").map(str::trim)
            && is_name(name)
        {
            let modules = crate::help::scope(
                (!prelude.is_empty()).then(|| crate::help::Module::program(&prelude.join("\n\n"))),
                &vault.extensions.module_roots,
            );
            if let Some(text) = crate::help::types(name, &modules) {
                write!(out, "{text}").map_err(|error| error.to_string())?;
                continue;
            }
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
            crate::run_on(&source, vault, mode, backend)
        };
        let origin = format!("<repl:{typed}>");
        let at = source.len() - line.len();
        let mut shown_outcome = outcome.clone();
        for diagnostic in &mut shown_outcome.diagnostics {
            if diagnostic.origin.is_none() && diagnostic.span.start >= at {
                diagnostic.span = diagnostic.span.start - at..diagnostic.span.end - at;
                diagnostic.origin = Some(origin.clone());
            }
        }
        emit(
            &shown_outcome,
            line,
            &origin,
            HQL,
            options,
            vault,
            shown,
            out,
            err,
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
                    // A doc string and the declarations it documents are one
                    // input, ended by an empty line, as a file has them.
                    if opens_doc(&text) {
                        if text
                            .lines()
                            .last()
                            .is_some_and(|last| last.trim().is_empty())
                        {
                            return Ok(Some(text));
                        }
                        continue;
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
        let input = context.input();
        let documented = opens_doc(input) && !input.ends_with('\n');
        Ok(if is_open(input) || documented {
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

/// Whether an input opens with a doc line, and so runs to an empty line.
fn opens_doc(input: &str) -> bool {
    input
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim_start().starts_with("///"))
}

/// Whether a word is a name, bare or qualified, rather than an expression.
fn is_name(word: &str) -> bool {
    !word.is_empty()
        && word.split('.').all(|segment| {
            segment.starts_with(|c: char| c.is_alphabetic() || c == '_')
                && segment.chars().all(|c| c.is_alphanumeric() || c == '_')
        })
}

/// The session's inputs and a new one as one source, an empty line between
/// each, so a doc string in one opens after an empty line.
fn join(prelude: &[String], line: &str) -> String {
    let mut source = prelude.join("\n\n");
    if !source.is_empty() {
        source.push_str("\n\n");
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
