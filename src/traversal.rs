//! The lazy carrier behind a generated `Seq<T>` or `Stream<T>`.
//!
//! A traversal is a factory, never a live cursor: it says how to start
//! producing values, so two uses of one binding see the same values. A
//! consumer drives it by supplying a sink that answers, for each value,
//! whether it wants another; a sink that stops is what lets `take` end a
//! traversal that would not end by itself. A finite traversal is forced once,
//! when a step needs its whole input, and keeps what it produced.

use crate::execution::BlockStmt;
use crate::types::{TypeRef, Value, collections};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

/// A restartable producer of values in a stable order.
#[derive(Debug)]
pub struct Traversal {
    /// The type of each value.
    pub element: TypeRef,
    /// Whether production ends: a `Seq<T>` rather than a `Stream<T>`.
    pub finite: bool,
    pub(crate) source: Source,
    forced: OnceLock<Vec<Value>>,
}

/// Where a traversal's values come from.
#[derive(Debug)]
pub(crate) enum Source {
    /// The block body of an iterating function, with the scope it runs in.
    Block {
        statements: Arc<Vec<BlockStmt>>,
        scope: HashMap<String, Value>,
    },
    /// The vault's cards, in the order a sorted walk reaches them.
    Cards,
    /// Each value of `input` passed through a function.
    Map {
        input: Arc<Traversal>,
        by: Value,
        span: Range<usize>,
    },
    /// The values of `input` a predicate accepts.
    Filter {
        input: Arc<Traversal>,
        by: Value,
        span: Range<usize>,
    },
    /// The first `n` values of `input`.
    Take { input: Arc<Traversal>, n: usize },
    /// The values of `input` after the first `n`.
    Drop { input: Arc<Traversal>, n: usize },
    /// Consecutive values of `input` in lists of `n`.
    Collect { input: Arc<Traversal>, n: usize },
    /// The members of each sequence `input` produces, in order.
    Emit { input: Arc<Traversal> },
    /// Each value of `input` written as one piece of text in `format`.
    Format {
        input: Arc<Traversal>,
        format: crate::types::TypeConstructor,
        span: Range<usize>,
    },
    /// Values already produced, kept at a sequence type.
    Kept,
    /// Each document of `input` decoded as the traversal's element type, a
    /// declared record the read is expected to give.
    Decode {
        input: Arc<Traversal>,
        leaves: crate::types::Leaves,
        /// The endpoint read, as an error names it.
        from: String,
        span: Range<usize>,
    },
    /// Documents read from an endpoint as they arrive.
    Read {
        reading: Arc<crate::io::Reading>,
        span: Range<usize>,
    },
}

impl Traversal {
    pub(crate) fn new(element: TypeRef, finite: bool, source: Source) -> Self {
        Self {
            element,
            finite,
            source,
            forced: OnceLock::new(),
        }
    }

    /// A sequence of values already produced.
    pub(crate) fn kept(element: TypeRef, values: Vec<Value>) -> Self {
        let traversal = Self::new(element, true, Source::Kept);
        traversal.keep(values);
        traversal
    }

    /// `Seq<T>` when production ends, `Stream<T>` when it need not.
    #[must_use]
    pub fn type_of(&self) -> TypeRef {
        let constructor = if self.finite {
            collections::SEQ
        } else {
            collections::STREAM
        };
        constructor.apply([self.element.clone()])
    }

    /// The values, once a step has needed all of them.
    #[must_use]
    pub fn forced(&self) -> Option<&[Value]> {
        self.forced.get().map(Vec::as_slice)
    }

    pub(crate) fn keep(&self, values: Vec<Value>) {
        // A second forcing produced the same values, so either is kept.
        let _ = self.forced.set(values);
    }
}
