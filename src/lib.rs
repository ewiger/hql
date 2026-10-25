//! HQL — Hyper Query Language: a typed language for querying and computing
//! over structured knowledge.
//!
//! The core is pure and names no source. A [`vault::Vault`] is one source of
//! documents, supplied from outside; a program without one still runs, and
//! anything needing a vault says so rather than pretending to have found
//! nothing.
//!
//! Running a program yields an [`Outcome`]: a value, and the contents of the
//! run's diagnostic channel (HQL-0045). The diagnostics come back beside the
//! value rather than instead of it, so a caller can walk them afterwards.

#![forbid(unsafe_code)]

mod ast;
mod checker;
mod constructors;
mod declarations;
mod evaluation;
mod execution;
mod knowledge;
mod lexer;
pub mod modules;
pub mod naming;
mod net;
mod parser;

pub mod backends;
pub mod boundary;
pub mod builtins;
pub mod cli;
pub mod data;
pub mod decimal;
pub mod diagnostics;
pub mod docs;
pub mod document;
pub mod extensions;
pub mod graph;
pub mod help;
pub mod io;
pub mod library;
pub mod reference;
pub mod render;
pub mod reporting;
pub mod search;
pub mod sources;
pub mod transclude;
pub mod traversal;
pub mod types;
pub mod vault;

use diagnostics::{Diagnostic, Disposition, Kind, Phase, Severity};
use reporting::Mode;
use types::{TypeRef, Value};
use vault::Vault;

/// What running a program produced: a value, if one survived, and the run's
/// diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The value of the program's last expression, absent when it did not run.
    pub value: Option<Value>,
    /// The type of that expression, when checking got far enough to know.
    pub inferred: Option<TypeRef>,
    /// The diagnostic channel's contents, in the order they were written.
    pub diagnostics: Vec<Diagnostic>,
}

impl Outcome {
    /// Whether the channel holds a diagnostic at or above `error`, the fail
    /// level unless configured.
    #[must_use]
    pub fn failed(&self) -> bool {
        diagnostics::reaches(&self.diagnostics, Severity::Error)
    }
}

/// A run's hold on the diagnostic channel. The run that opened the channel
/// closes it; a run inside one that is already open, a host's, writes into
/// that channel and leaves it open.
struct Run {
    owner: bool,
}

impl Run {
    fn open(vault: &Vault) -> Self {
        let owner = io::open_diagnostics();
        // The loader ran before this run: what it wrote is the first thing on
        // the channel, unless a host already wrote it there.
        if owner {
            for diagnostic in &vault.warnings {
                io::write_diagnostic(diagnostic.clone());
            }
        }
        Self { owner }
    }

    fn write(&self, diagnostic: Diagnostic) {
        io::write_diagnostic(diagnostic);
    }

    fn finish(self, value: Option<Value>, inferred: Option<TypeRef>) -> Outcome {
        Outcome {
            value,
            inferred,
            diagnostics: if self.owner {
                io::close_diagnostics()
            } else {
                io::diagnostics_written()
            },
        }
    }
}

/// A diagnostic as the phase that produced it states it. A backend's
/// refusal reaches the driver through execution and stays the planner's.
fn produced_by(phase: Phase, diagnostic: Diagnostic) -> Diagnostic {
    if phase == Phase::Execute && diagnostic.phase == Phase::Plan {
        return diagnostic;
    }
    diagnostic.in_phase(phase)
}

/// Parse a program, its failure the parser's.
fn parsed(source: &str) -> Result<ast::Program, Diagnostic> {
    parser::parse(source).map_err(|diagnostic| produced_by(Phase::Parse, diagnostic))
}

/// Check a program under `strict`, its failure the checker's.
fn checked(program: &ast::Program, vault: &Vault) -> Result<execution::Program, Diagnostic> {
    checker::check(program, vault)
        .map_err(|diagnostic| checked_diagnostic(diagnostic, Mode::Strict))
}

/// A checker's diagnostic: a module's syntax failure keeps its parser's
/// phase, and under `collect` the checker went on past the statement.
fn checked_diagnostic(diagnostic: Diagnostic, mode: Mode) -> Diagnostic {
    if diagnostic.phase == Phase::Parse {
        return diagnostic;
    }
    let diagnostic = produced_by(Phase::Check, diagnostic);
    match (mode, diagnostic.disposition) {
        (Mode::Collect, Disposition::Fatal) => {
            diagnostic.with_disposition(Disposition::Recoverable)
        }
        _ => diagnostic,
    }
}

/// Parse and type-check a program without evaluating it, with no vault.
///
/// # Errors
///
/// Returns the first syntax, type or name failure.
pub fn check(source: &str) -> Result<TypeRef, Diagnostic> {
    check_in(source, &Vault::empty())
}

/// Parse and type-check a program against a vault, without evaluating it.
///
/// # Errors
///
/// Returns the first syntax, type or name failure.
pub fn check_in(source: &str, vault: &Vault) -> Result<TypeRef, Diagnostic> {
    checked(&parsed(source)?, vault).map(|program| program.result)
}

/// The steps of the pipeline a program's last expression ends, first to
/// last, each with the shape it was checked to: what a planner may assume
/// about splitting it. A direct call is a one-step pipeline; an expression
/// inside a step's arguments, a receiver call in a lambda among them, is
/// evaluated within that step and is not one.
///
/// # Errors
///
/// Returns the first syntax, type or name failure.
pub fn pipeline(source: &str) -> Result<Vec<(String, extensions::Shape)>, Diagnostic> {
    let program = checked(&parsed(source)?, &Vault::empty())?;
    let last = program
        .statements
        .iter()
        .rev()
        .find_map(|statement| match statement {
            execution::Stmt::Expr(expression) => Some(expression),
            _ => None,
        });
    Ok(last
        .map(execution::Expr::pipeline)
        .unwrap_or_default()
        .into_iter()
        .map(|(name, shape)| (name.to_owned(), shape))
        .collect())
}

/// One step of a planned pipeline: the operation it invokes, the shape and
/// purity the planner read off the operation's type, and the stage
/// annotation it wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedStep {
    /// The operation the step invokes, a program's own function among them.
    pub name: String,
    /// How the operation may be split.
    pub shape: extensions::Shape,
    /// Whether the operation may be moved.
    pub purity: extensions::Purity,
    /// The stage annotation.
    pub stage: backends::Stage,
    /// `fan-out` or `fan-in` for a step that joins or splits the dataflow
    /// graph (HQL-0037).
    pub topology: Option<&'static str>,
    /// The branches of a `fork` or a `tee`, each by its name, which read
    /// the step's own input.
    pub branches: Vec<PlannedBranch>,
}

/// One branch of a fan-out step: its name and its steps over the input the
/// step reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedBranch {
    /// The name the branch is bound by.
    pub name: String,
    /// Its steps, first to last.
    pub steps: Vec<PlannedStep>,
}

/// A planned pipeline: the expression its first step reads, and its steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPipeline {
    /// Where the pipeline is written, as a byte range of the source.
    pub span: std::ops::Range<usize>,
    /// The source text of the expression the first step reads.
    pub input: String,
    /// The steps, first to last.
    pub steps: Vec<PlannedStep>,
}

/// The plan of a program on a backend: each pipeline a statement of the
/// program is, with every step annotated by the stage the planner gave it.
/// A pipeline inside a step's arguments, or in a function's body, is
/// planned too, and not listed.
///
/// # Errors
///
/// Returns the first syntax, type or name failure.
pub fn plan(
    source: &str,
    vault: &Vault,
    backend: &backends::Registered,
) -> Result<Vec<PlannedPipeline>, Diagnostic> {
    let program = checked(&parsed(source)?, vault)?;
    backend.plan(&program);
    let mut planned = Vec::new();
    for statement in &program.statements {
        let (execution::Stmt::Bind { value, .. } | execution::Stmt::Expr(value)) = statement else {
            continue;
        };
        let (steps, input) = planned_steps(value);
        if steps.is_empty() {
            continue;
        }
        planned.push(PlannedPipeline {
            span: value.span.clone(),
            input: source
                .get(input.span.clone())
                .unwrap_or_default()
                .to_owned(),
            steps,
        });
    }
    Ok(planned)
}

/// What `hql boundary` prints of a configured source (HQL-0035): its
/// effective boundary for an element type, with the source's own policies.
#[derive(Debug, Clone)]
pub struct InspectedBoundary {
    /// The element type the boundary maps.
    pub element: String,
    /// The boundary, each property with its origin.
    pub boundary: boundary::EffectiveBoundary,
    /// How a write treats what the target holds.
    pub mode: sources::WriteMode,
}

/// The effective boundary of the configured source `id` read as
/// `element`, or as the type its configuration names, the type declared by
/// `types`, a program the boundary's type is checked in. The external
/// fields are read where the format knows them before reading: a CSV
/// file's header and a table's columns.
///
/// # Errors
///
/// Returns why the boundary cannot be inspected: the program does not
/// check, the source or the type is unknown, or every boundary error.
pub fn inspect_boundary(
    types: &str,
    vault: &Vault,
    id: &str,
    element: Option<&str>,
) -> Result<InspectedBoundary, String> {
    let program = checker::check(
        &parser::parse(types).map_err(|error| error.message())?,
        vault,
    )
    .map_err(|error| error.message())?;
    types::program::install(std::sync::Arc::clone(&program.lattice));
    let configured = vault.sources.get(id).ok_or_else(|| {
        format!("no source `{id}` is configured; a source is declared under `[sources.<id>]`")
    })?;
    let written = element
        .map(str::to_owned)
        .or_else(|| configured.element.clone())
        .ok_or_else(|| {
            format!("source `{id}` names no `element`; name the type it is read as with `--as`")
        })?;
    let ty = types::named(&written, &[])
        .ok_or_else(|| format!("`{written}` is not a type the program declares"))?;
    let external = match &configured.kind {
        sources::Kind::Sqlite { path, table, .. } => Some(sources::sqlite_columns(path, table)?),
        sources::Kind::File { path, format } if *format == "Csv" => {
            io::read_configured(path, types::TypeConstructor(format), false)?
                .first()
                .and_then(|row| match row {
                    Value::Data(tree) => match tree.as_ref() {
                        data::Data::Map(entries) => Some(entries.keys().cloned().collect()),
                        _ => None,
                    },
                    _ => None,
                })
        }
        _ => None,
    };
    let boundary = extensions::io::boundary_for(configured, &ty, external.as_deref())?
        .ok_or_else(|| format!("`{written}` is not a record, so it has no boundary"))?;
    Ok(InspectedBoundary {
        element: written,
        boundary,
        mode: configured.mode,
    })
}

/// The planned steps of the pipeline `end` ends, first to last, and the
/// expression the first reads; a `fork`'s or a `tee`'s branches with it.
fn planned_steps(end: &execution::Expr) -> (Vec<PlannedStep>, &execution::Expr) {
    let mut nodes = Vec::new();
    let mut at = end;
    while let Some((_, input)) = at.as_step() {
        nodes.push(at);
        at = input;
    }
    nodes.reverse();
    let steps = nodes
        .into_iter()
        .filter_map(|node| {
            let (plan, _) = node.as_step()?;
            let branches = match &node.kind {
                execution::Kind::Fork { branches, .. } => branches
                    .iter()
                    .map(|(name, branch)| PlannedBranch {
                        name: name.clone(),
                        steps: planned_steps(branch).0,
                    })
                    .collect(),
                execution::Kind::Tee { name, branch, .. } => vec![PlannedBranch {
                    name: name.clone(),
                    steps: planned_steps(branch).0,
                }],
                _ => Vec::new(),
            };
            Some(PlannedStep {
                name: plan.invokes.name.clone(),
                shape: plan.invokes.shape,
                purity: plan.invokes.purity,
                stage: *plan.stage.get()?,
                topology: backends::topology(&plan.invokes.name),
                branches,
            })
        })
        .collect();
    (steps, at)
}

/// Write a stream a program returned as it arrives: each value goes to
/// `sink`, which answers whether it wants another.
///
/// # Errors
///
/// Returns the first failure producing a value.
pub fn stream(
    traversal: &traversal::Traversal,
    vault: &Vault,
    mut sink: impl FnMut(Value) -> bool,
) -> Result<(), Diagnostic> {
    evaluation::traverse(traversal, vault, &mut sink)
}

/// Parse, type-check, then evaluate a program with no vault.
///
/// # Errors
///
/// Returns the first failure from any stage.
pub fn eval(source: &str) -> Result<Value, Diagnostic> {
    let program = parsed(source)?;
    let program = checked(&program, &Vault::empty())?;
    backends::LOCALHOST
        .run(&program, &Vault::empty(), Vec::new(), Mode::Strict)
        .0
        .map_err(|diagnostic| produced_by(Phase::Execute, diagnostic))
}

/// Type-check a program, reporting under a mode.
#[must_use]
pub fn check_reported(source: &str, vault: &Vault, mode: Mode) -> Outcome {
    let run = Run::open(vault);
    let program = match parser::parse(source) {
        Ok(program) => program,
        Err(diagnostic) => {
            run.write(produced_by(Phase::Parse, diagnostic));
            return run.finish(None, None);
        }
    };
    let (checked, found) = infer(&program, vault, mode);
    for diagnostic in found {
        run.write(diagnostic);
    }
    if let Some(program) = &checked {
        for warning in &program.warnings {
            run.write(warning.clone());
        }
        // Every alias's signature is visible, since the checker wrote it.
        for (name, signature) in &program.signatures {
            run.write(Diagnostic::new(
                Phase::Check,
                Kind::Signature,
                Severity::Info,
                Disposition::Warning,
                0..0,
                match types::builtin::type_variable("T", Some(&TypeRef::ORDERABLE)) {
                    Some(ordered) if signature.mentions(&ordered) => {
                        format!("{name} : {signature} where T : Orderable")
                    }
                    _ => format!("{name} : {signature}"),
                },
            ));
        }
        // So is every function that reads or writes outside its result.
        for (name, uses) in &program.effects {
            let uses: Vec<String> = uses.iter().map(|used| format!("`{used}`")).collect();
            run.write(Diagnostic::new(
                Phase::Check,
                Kind::Effect,
                Severity::Info,
                Disposition::Warning,
                0..0,
                format!("{name} is Effectful: it uses {}", uses.join(", ")),
            ));
        }
    }
    run.finish(None, checked.map(|program| program.result))
}

/// Run a program against a vault under a reporting mode, on `localhost`.
///
/// Checking runs over the whole program before anything is evaluated. In
/// [`Mode::Collect`] every statement is checked even after one fails, so the
/// channel holds the whole list rather than only the first diagnostic.
#[must_use]
pub fn run(source: &str, vault: &Vault, mode: Mode) -> Outcome {
    run_on(source, vault, mode, backends::Selection::standard())
}

/// Run a program against a vault under a reporting mode, on a selected
/// backend. The backend receives the checked program and nothing earlier:
/// parsing and checking have run before it is reached. Every phase writes
/// into the run's diagnostic channel, opened here unless a host holds it
/// open.
#[must_use]
pub fn run_on(source: &str, vault: &Vault, mode: Mode, selection: backends::Selection) -> Outcome {
    let run = Run::open(vault);
    let program = match parser::parse(source) {
        Ok(program) => program,
        Err(diagnostic) => {
            run.write(produced_by(Phase::Parse, diagnostic));
            return run.finish(None, None);
        }
    };
    let (checked, found) = infer(&program, vault, mode);
    let refused = found.iter().any(Diagnostic::is_error);
    for diagnostic in found {
        run.write(diagnostic);
    }
    // A check error ends the phase, and later phases do not start.
    let Some(program) = checked.filter(|_| !refused) else {
        return run.finish(None, None);
    };
    for warning in &program.warnings {
        run.write(warning.clone());
    }
    let inferred = program.result.clone();
    let (value, log) = selection.backend.run(&program, vault, Vec::new(), mode);
    for entry in log {
        run.write(produced_by(Phase::Execute, entry));
    }
    match value {
        Ok(value) => run.finish(Some(value), Some(inferred)),
        Err(diagnostic) => {
            run.write(produced_by(Phase::Execute, diagnostic));
            run.finish(None, Some(inferred))
        }
    }
}

/// Check one cell of a document with the types of the cells it reads,
/// giving the cell's diagnostics with spans into its body.
pub(crate) fn check_cell(
    source: &str,
    vault: &Vault,
    mode: Mode,
    bound: &[(String, TypeRef)],
) -> (Option<execution::Program>, Vec<Diagnostic>) {
    let program = match parser::parse(source) {
        Ok(program) => program,
        Err(diagnostic) => return (None, vec![produced_by(Phase::Parse, diagnostic)]),
    };
    let (checked, mut found) = match mode {
        Mode::Strict => match checker::check_with(&program, vault, bound) {
            Ok(program) => (Some(program), Vec::new()),
            Err(diagnostic) => (None, vec![checked_diagnostic(diagnostic, mode)]),
        },
        Mode::Collect => {
            let (checked, found) = checker::check_collecting_with(&program, vault, bound);
            (
                checked,
                found
                    .into_iter()
                    .map(|diagnostic| checked_diagnostic(diagnostic, mode))
                    .collect(),
            )
        }
    };
    if found.iter().any(Diagnostic::is_error) {
        return (None, found);
    }
    if let Some(program) = &checked {
        found.extend(program.warnings.iter().cloned());
    }
    (checked, found)
}

/// Evaluate one checked cell with the values of the cells it reads, on
/// `localhost`.
pub(crate) fn evaluate_cell(
    program: &execution::Program,
    vault: &Vault,
    bound: Vec<(String, Value)>,
    mode: Mode,
) -> (Result<Value, Diagnostic>, Vec<Diagnostic>) {
    let (value, log) = backends::LOCALHOST.run(program, vault, bound, mode);
    (
        value.map_err(|diagnostic| produced_by(Phase::Execute, diagnostic)),
        log.into_iter()
            .map(|entry| produced_by(Phase::Execute, entry))
            .collect(),
    )
}

fn infer(
    program: &ast::Program,
    vault: &Vault,
    mode: Mode,
) -> (Option<execution::Program>, Vec<Diagnostic>) {
    let (checked, found) = match mode {
        Mode::Strict => match checker::check(program, vault) {
            Ok(program) => (Some(program), Vec::new()),
            Err(diagnostic) => (None, vec![diagnostic]),
        },
        Mode::Collect => checker::check_collecting(program, vault),
    };
    (
        checked,
        found
            .into_iter()
            .map(|diagnostic| checked_diagnostic(diagnostic, mode))
            .collect(),
    )
}
