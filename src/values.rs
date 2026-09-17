//! Runtime values returned by the pure HQL core.

use std::fmt;

/// A result of evaluating a bootstrap expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// A signed 64-bit integer.
    Int(i64),
    /// A finite double-precision number.
    Float(f64),
    /// A Boolean truth value.
    Bool(bool),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => write!(f, "{value}"),
            // Debug formatting keeps the decimal point, so 1.0 is not printed as 1.
            Self::Float(value) => write!(f, "{value:?}"),
            Self::Bool(value) => write!(f, "{value}"),
        }
    }
}
