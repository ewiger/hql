//! Runtime values returned by the pure HQL core.

use std::fmt;

/// A result of evaluating a bootstrap expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    /// A signed 64-bit integer.
    Int(i64),
    /// A Boolean truth value.
    Bool(bool),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => write!(f, "{value}"),
            Self::Bool(value) => write!(f, "{value}"),
        }
    }
}
