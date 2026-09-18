//! Structured failures with zero-based, half-open UTF-8 byte ranges.

use std::ops::Range;

/// An expected syntax, type, name, or runtime failure in an HQL program.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Diagnostic {
    /// The source is outside the grammar or an implementation limit.
    #[error("syntax error at bytes {span:?}: {message}")]
    Syntax {
        /// The affected byte range; an empty range denotes end of input.
        span: Range<usize>,
        /// A human-readable explanation.
        message: String,
    },
    /// An expression has a type its position does not accept.
    #[error("type error at bytes {span:?}: {message}")]
    Type {
        /// The range of the offending expression.
        span: Range<usize>,
        /// A human-readable explanation.
        message: String,
    },
    /// A name is not bound, or a field does not exist on a value.
    #[error("name error at bytes {span:?}: {message}")]
    Name {
        /// The range of the unresolved name.
        span: Range<usize>,
        /// A human-readable explanation.
        message: String,
    },
    /// An operation failed while evaluating, with no static fault.
    #[error("evaluation error at bytes {span:?}: {message}")]
    Runtime {
        /// The range of the failing expression.
        span: Range<usize>,
        /// A human-readable explanation.
        message: String,
    },
    /// An addition left the bootstrap `i64` or finite `f64` range.
    #[error("evaluation error at bytes {span:?}: numeric overflow")]
    Overflow {
        /// The range of the addition expression.
        span: Range<usize>,
    },
}

impl Diagnostic {
    /// The source range the diagnostic points at.
    #[must_use]
    pub fn span(&self) -> Range<usize> {
        match self {
            Self::Syntax { span, .. }
            | Self::Type { span, .. }
            | Self::Name { span, .. }
            | Self::Runtime { span, .. }
            | Self::Overflow { span } => span.clone(),
        }
    }

    /// The stage that produced the diagnostic, as a stable lowercase word.
    #[must_use]
    pub fn stage(&self) -> &'static str {
        match self {
            Self::Syntax { .. } => "syntax",
            Self::Type { .. } => "type",
            Self::Name { .. } => "name",
            Self::Runtime { .. } | Self::Overflow { .. } => "evaluation",
        }
    }

    /// The explanation without the stage prefix or the byte range.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Syntax { message, .. }
            | Self::Type { message, .. }
            | Self::Name { message, .. }
            | Self::Runtime { message, .. } => message.clone(),
            Self::Overflow { .. } => "numeric overflow".to_owned(),
        }
    }

    pub(crate) fn syntax(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::Syntax {
            span,
            message: message.into(),
        }
    }

    pub(crate) fn typing(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::Type {
            span,
            message: message.into(),
        }
    }

    pub(crate) fn name(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::Name {
            span,
            message: message.into(),
        }
    }

    pub(crate) fn runtime(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::Runtime {
            span,
            message: message.into(),
        }
    }
}
