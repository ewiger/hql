//! HQL — Hyper Query Language: a typed language for querying and computing
//! over structured knowledge.
//!
//! The core is pure and names no source. A [`vault::Vault`] is one source of
//! documents, supplied from outside; a program without one still runs, and
//! anything needing a vault says so rather than pretending to have found
//! nothing.
//!
//! Running a program yields an [`Outcome`]: a value, and a queue of everything
//! the run had to say. The queue comes back beside the value rather than
//! instead of it, so a caller can walk it afterwards.

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

pub mod builtins;
pub mod cli;
pub mod data;
pub mod diagnostics;
pub mod document;
pub mod extensions;
pub mod graph;
pub mod io;
pub mod library;
pub mod render;
pub mod reporting;
pub mod search;
pub mod sources;
pub mod transclude;
pub mod traversal;
pub mod types;
pub mod vault;
pub mod warnings;

use diagnostics::Diagnostic;
use reporting::{Mode, Report, Reports};
use types::{TypeRef, Value};
use vault::Vault;

/// What running a program produced: a value, if one survived, and the queue.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The value of the program's last expression, absent when it did not run.
    pub value: Option<Value>,
    /// The type of that expression, when checking got far enough to know.
    pub inferred: Option<TypeRef>,
    /// Everything the run had to say, oldest first.
    pub reports: Reports,
}

impl Outcome {
    /// Whether anything queued makes the result untrustworthy.
    #[must_use]
    pub fn failed(&self) -> bool {
        self.reports.has_errors()
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
    checker::check(&parser::parse(source)?, vault).map(|program| program.result)
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
    let program = parser::parse(source)?;
    let program = checker::check(&program, &Vault::empty())?;
    evaluation::evaluate(&program, &Vault::empty()).0
}

/// Type-check a program, reporting under a mode.
#[must_use]
pub fn check_reported(source: &str, vault: &Vault, mode: Mode) -> Outcome {
    let program = match parser::parse(source) {
        Ok(program) => program,
        Err(diagnostic) => return failed(&diagnostic),
    };
    let (checked, found) = infer(&program, vault, mode);
    let mut reports: Reports = found.iter().map(Report::of).collect();
    if let Some(program) = &checked {
        for warning in &program.warnings {
            reports.push(Report::of_check_warning(warning));
        }
        // Every alias's signature is visible, since the checker wrote it.
        for (name, signature) in &program.signatures {
            reports.push(Report {
                severity: reporting::Severity::Info,
                stage: "type".to_owned(),
                message: match types::builtin::type_variable("T", Some(&TypeRef::ORDERABLE)) {
                    Some(ordered) if signature.mentions(&ordered) => {
                        format!("{name} : {signature} where T : Orderable")
                    }
                    _ => format!("{name} : {signature}"),
                },
                code: Some("signature".to_owned()),
                ..Report::of_check_warning(&warnings::Warning::new(0..0, ""))
            });
        }
        // So is every function that reads or writes outside its result.
        for (name, uses) in &program.effects {
            let uses: Vec<String> = uses.iter().map(|used| format!("`{used}`")).collect();
            reports.push(Report {
                severity: reporting::Severity::Info,
                stage: "type".to_owned(),
                message: format!("{name} is Effectful: it uses {}", uses.join(", ")),
                code: Some("effect".to_owned()),
                ..Report::of_check_warning(&warnings::Warning::new(0..0, ""))
            });
        }
    }
    Outcome {
        value: None,
        inferred: checked.map(|program| program.result),
        reports,
    }
}

/// Run a program against a vault under a reporting mode.
///
/// Checking runs over the whole program before anything is evaluated. In
/// [`Mode::Collect`] every statement is checked even after one fails, so the
/// queue holds the whole list rather than only the first entry.
#[must_use]
pub fn run(source: &str, vault: &Vault, mode: Mode) -> Outcome {
    let program = match parser::parse(source) {
        Ok(program) => program,
        Err(diagnostic) => return failed(&diagnostic),
    };
    let (checked, found) = infer(&program, vault, mode);
    let mut reports: Reports = found.iter().map(Report::of).collect();
    if reports.has_errors() {
        return Outcome {
            value: None,
            inferred: None,
            reports,
        };
    }
    let Some(program) = checked else {
        return Outcome {
            value: None,
            inferred: None,
            reports,
        };
    };
    for warning in &program.warnings {
        reports.push(Report::of_check_warning(warning));
    }
    let inferred = program.result.clone();
    let (value, log) = evaluation::evaluate(&program, vault);
    for entry in log {
        reports.push(entry);
    }
    match value {
        Ok(value) => Outcome {
            value: Some(value),
            inferred: Some(inferred),
            reports,
        },
        Err(diagnostic) => {
            reports.push(Report::of(&diagnostic));
            Outcome {
                value: None,
                inferred: Some(inferred),
                reports,
            }
        }
    }
}

/// Check one cell of a document with the types of the cells it reads.
pub(crate) fn check_cell(
    source: &str,
    vault: &Vault,
    mode: Mode,
    bound: &[(String, TypeRef)],
) -> (Option<execution::Program>, Reports) {
    let program = match parser::parse(source) {
        Ok(program) => program,
        Err(diagnostic) => return (None, std::iter::once(Report::of(&diagnostic)).collect()),
    };
    let (checked, found) = match mode {
        Mode::Strict => match checker::check_with(&program, vault, bound) {
            Ok(program) => (Some(program), Vec::new()),
            Err(diagnostic) => (None, vec![diagnostic]),
        },
        Mode::Collect => checker::check_collecting_with(&program, vault, bound),
    };
    let mut reports: Reports = found.iter().map(Report::of).collect();
    if reports.has_errors() {
        return (None, reports);
    }
    if let Some(program) = &checked {
        for warning in &program.warnings {
            reports.push(Report::of_check_warning(warning));
        }
    }
    (checked, reports)
}

/// Evaluate one checked cell with the values of the cells it reads.
pub(crate) fn evaluate_cell(
    program: &execution::Program,
    vault: &Vault,
    bound: Vec<(String, Value)>,
) -> (Result<Value, Diagnostic>, Reports) {
    let (value, log) = evaluation::evaluate_with(program, vault, bound);
    (value, log.into_iter().collect())
}

fn infer(
    program: &ast::Program,
    vault: &Vault,
    mode: Mode,
) -> (Option<execution::Program>, Vec<Diagnostic>) {
    match mode {
        Mode::Strict => match checker::check(program, vault) {
            Ok(program) => (Some(program), Vec::new()),
            Err(diagnostic) => (None, vec![diagnostic]),
        },
        Mode::Collect => checker::check_collecting(program, vault),
    }
}

fn failed(diagnostic: &Diagnostic) -> Outcome {
    Outcome {
        value: None,
        inferred: None,
        reports: std::iter::once(Report::of(diagnostic)).collect(),
    }
}
