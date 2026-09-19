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
mod parser;

pub mod builtins;
pub mod cli;
pub mod data;
pub mod diagnostics;
pub mod document;
pub mod extensions;
pub mod graph;
pub mod render;
pub mod reporting;
pub mod search;
pub mod sources;
pub mod transclude;
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

/// Parse, type-check, then evaluate a program with no vault.
///
/// # Errors
///
/// Returns the first failure from any stage.
pub fn eval(source: &str) -> Result<Value, Diagnostic> {
    let program = parser::parse(source)?;
    let program = checker::check(&program, &Vault::empty())?;
    evaluation::evaluate(&program, &Vault::empty()).map(|(value, _)| value)
}

/// Type-check a program, reporting under a mode.
#[must_use]
pub fn check_reported(source: &str, vault: &Vault, mode: Mode) -> Outcome {
    let program = match parser::parse(source) {
        Ok(program) => program,
        Err(diagnostic) => return failed(&diagnostic),
    };
    let (checked, found) = infer(&program, vault, mode);
    let reports: Reports = found.iter().map(Report::of).collect();
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
    let inferred = program.result.clone();
    match evaluation::evaluate(&program, vault) {
        Ok((value, warnings)) => {
            for warning in &warnings {
                reports.push(Report::of_warning(warning));
            }
            Outcome {
                value: Some(value),
                inferred: Some(inferred),
                reports,
            }
        }
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
