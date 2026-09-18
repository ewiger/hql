//! Non-fatal reports: work continued, but something is worth saying.
//!
//! A warning never changes an exit status. It exists because an unresolved
//! document reference is permitted — a forward link to a document nobody has
//! written yet is ordinary — while silently yielding absence would hide it.

use std::ops::Range;

/// Something worth reporting that did not stop evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// The range of the expression that produced it.
    pub span: Range<usize>,
    /// A human-readable explanation.
    pub message: String,
}

impl Warning {
    pub(crate) fn new(span: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "warning at bytes {:?}: {}", self.span, self.message)
    }
}
