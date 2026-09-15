//! HQL — Hyper Query Language: a pure, typed expression core.

#![forbid(unsafe_code)]

mod ast;
pub mod diagnostics;
mod evaluation;
mod parser;
pub mod types;
pub mod values;

use diagnostics::Diagnostic;
use types::Type;
use values::Value;

/// Parse and type-check one complete expression without evaluating it.
pub fn check(source: &str) -> Result<Type, Diagnostic> {
    types::infer(&parser::parse(source)?)
}

/// Parse, type-check, then evaluate one complete expression.
pub fn eval(source: &str) -> Result<Value, Diagnostic> {
    let expression = parser::parse(source)?;
    types::infer(&expression)?;
    evaluation::evaluate(&expression)
}
