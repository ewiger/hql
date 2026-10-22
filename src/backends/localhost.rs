//! `localhost`: the standard backend, the in-memory evaluator.
//!
//! A tree walk over the execution tree, each step run to completion before
//! the next, one traversal per step. It applies none of the rewrites a
//! step's shape permits: it is defined by its result, and the simplest
//! execution is the one least likely to be wrong about it. Its plan is the
//! trivial one: the whole pipeline is one stage, in process.

use super::{Backend, Execution};
use crate::diagnostics::Diagnostic;
use crate::execution::Program;
use crate::reporting::Report;
use crate::types::Value;
use crate::vault::Vault;

/// The in-memory evaluator, as a backend.
pub(super) struct Localhost;

impl Backend for Localhost {
    fn execution(&self) -> Execution {
        Execution::InProcess
    }

    fn run(
        &self,
        program: &Program,
        vault: &Vault,
        bound: Vec<(String, Value)>,
    ) -> (Result<Value, Diagnostic>, Vec<Report>) {
        crate::evaluation::evaluate_with(program, vault, bound)
    }
}
