//! Bootstrap types and static checking, independent of evaluation.

use crate::ast::{Expr, Kind};
use crate::diagnostics::Diagnostic;
use std::fmt;

/// A type supported by the bootstrap evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// A signed 64-bit integer, provisionally.
    Int,
    /// A Boolean truth value.
    Bool,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => f.write_str("Int"),
            Self::Bool => f.write_str("Bool"),
        }
    }
}

pub(crate) fn infer(expression: &Expr) -> Result<Type, Diagnostic> {
    match &expression.kind {
        Kind::Int(_) => Ok(Type::Int),
        Kind::Bool(_) => Ok(Type::Bool),
        Kind::Add(left, right) => {
            for operand in [left, right] {
                if infer(operand)? != Type::Int {
                    return Err(Diagnostic::Type {
                        span: operand.span.clone(),
                    });
                }
            }
            Ok(Type::Int)
        }
    }
}
