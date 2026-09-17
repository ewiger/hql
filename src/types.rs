//! Bootstrap types and static checking, independent of evaluation.

use crate::ast::{Expr, Kind};
use crate::diagnostics::Diagnostic;
use std::fmt;

/// A type supported by the bootstrap evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// A signed 64-bit integer, provisionally.
    Int,
    /// A double-precision number, provisionally.
    Float,
    /// A Boolean truth value.
    Bool,
}

impl Type {
    /// Whether the type is one that addition accepts.
    pub(crate) fn is_numeric(self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => f.write_str("Int"),
            Self::Float => f.write_str("Float"),
            Self::Bool => f.write_str("Bool"),
        }
    }
}

pub(crate) fn infer(expression: &Expr) -> Result<Type, Diagnostic> {
    match &expression.kind {
        Kind::Int(_) => Ok(Type::Int),
        Kind::Float(_) => Ok(Type::Float),
        Kind::Bool(_) => Ok(Type::Bool),
        Kind::Add(left, right) => {
            // Addition is homogeneous: there is no implicit Int-to-Float widening.
            let expected = infer(left)?;
            if !expected.is_numeric() {
                return Err(Diagnostic::Type {
                    span: left.span.clone(),
                });
            }
            if infer(right)? != expected {
                return Err(Diagnostic::Type {
                    span: right.span.clone(),
                });
            }
            Ok(expected)
        }
    }
}
