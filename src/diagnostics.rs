//! Structured failures with zero-based, half-open UTF-8 byte ranges.

use std::ops::Range;

/// An expected syntax, type, or runtime failure in an HQL expression.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Diagnostic {
    /// The source is outside the bootstrap grammar or implementation limits.
    #[error("syntax error at bytes {span:?}: {message}")]
    Syntax {
        /// The affected byte range; an empty range denotes end of input.
        span: Range<usize>,
        /// A human-readable explanation.
        message: String,
    },
    /// Addition requires two integer operands.
    #[error("type error at bytes {span:?}: addition requires Int operands")]
    Type {
        /// The range of the operand with the wrong type.
        span: Range<usize>,
    },
    /// An integer addition exceeded the bootstrap integer range.
    #[error("evaluation error at bytes {span:?}: integer overflow")]
    Overflow {
        /// The range of the addition expression.
        span: Range<usize>,
    },
}
