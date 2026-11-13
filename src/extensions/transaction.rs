//! The module `io.transaction`: the control calls of a transaction scope's
//! handle, `tx.commit(value)`, `tx.rollback(reason)` and `tx.retry()`.
//!
//! Each ends the attempt it is written in: it records the outcome asked for
//! and leaves the block by the one route an evaluation has, a failure,
//! which the scope recognizes as its own and acts on. See
//! [HQL-0051](../../doc/proposals/HQL-0051/README.md).

use super::{EvalCx, Operation, Purity, Signature};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::transaction::Control;
use crate::types::Value;
use std::ops::Range;

const fn control(name: &'static str, eval: super::Eval) -> Operation {
    Operation {
        name,
        spec: Signature::Declared,
        purity: Purity::Effectful,
        shape: super::Shape::Barrier,
        check: None,
        eval,
    }
}

/// Every operation `io.transaction` provides.
pub(crate) static STEPS: &[Operation] = &[
    control("commit", eval_commit),
    control("rollback", eval_rollback),
    control("retry", eval_retry),
];

fn leave(control: Control, name: &str, span: Range<usize>) -> Result<Value, Diagnostic> {
    if !crate::transaction::in_scope() {
        return Err(Diagnostic::runtime(
            span,
            format!("`tx.{name}` ends the transaction scope it is written in, and none is open"),
        ));
    }
    crate::transaction::set_control(control);
    Err(crate::transaction::leaving(span))
}

fn eval_commit(
    cx: &mut dyn EvalCx,
    _: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let value = match super::named_or_first(arguments, "value") {
        Some(argument) => cx.evaluate(&argument.value)?,
        None => {
            return Err(Diagnostic::runtime(
                span,
                "`commit` takes the value the scope commits with",
            ));
        }
    };
    leave(Control::Commit(value), "commit", span)
}

fn eval_rollback(
    cx: &mut dyn EvalCx,
    _: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let reason = match super::named_or_first(arguments, "reason") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Str(text) => text.to_string(),
            other => {
                return Err(Diagnostic::runtime(
                    span,
                    format!(
                        "`rollback` takes its reason as a String, not {}",
                        other.type_of()
                    ),
                ));
            }
        },
        None => return Err(Diagnostic::runtime(span, "`rollback` names its reason")),
    };
    leave(Control::Rollback(reason), "rollback", span)
}

fn eval_retry(
    _: &mut dyn EvalCx,
    _: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    leave(Control::Retry, "retry", span)
}
