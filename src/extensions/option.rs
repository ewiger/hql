//! The `option` module: the ways out of an `Option<T>`, each saying what
//! absence means. There is no unwrap.

use super::declared;
use super::{EvalCx, Operation, named_or_first};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::Value;
use std::ops::Range;
use std::sync::Arc;

/// Every step the `option` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    declared("is_some", "Whether a value is present.", eval_is_some),
    declared("is_none", "Whether the value is absent.", eval_is_none),
    declared(
        "unwrap_or",
        "The value when present, and a default otherwise.",
        eval_unwrap_or,
    ),
    declared(
        "unwrap_or_else",
        "The value when present, and what a function produces otherwise.",
        eval_unwrap_or_else,
    ),
    declared(
        "and_then",
        "A function over the present value, whose own absence is kept.",
        eval_and_then,
    ),
    declared(
        "or_else",
        "The value when present, and what a function produces otherwise, which may itself be absent.",
        eval_or_else,
    ),
    declared(
        "alternative",
        "The value when present, and another option otherwise.",
        eval_alternative,
    ),
];

/// The present value, or `None` for absence under any number of layers.
fn held(value: &Value) -> Option<Value> {
    match value {
        Value::Present(inner) => Some(inner.as_ref().clone()),
        Value::Absent(_) => None,
        other => Some(other.clone()),
    }
}

fn eval_is_some(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Bool(held(&input).is_some()))
}

fn eval_is_none(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Bool(held(&input).is_none()))
}

fn eval_unwrap_or(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Some(value) = held(&input) {
        return Ok(value);
    }
    let default = arguments
        .first()
        .ok_or_else(|| Diagnostic::runtime(span, "`unwrap_or` needs a default"))?;
    cx.evaluate(&default.value)
}

fn eval_unwrap_or_else(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Some(value) = held(&input) {
        return Ok(value);
    }
    let default = named_or_first(arguments, "default")
        .ok_or_else(|| Diagnostic::runtime(span, "`unwrap_or_else` needs a function"))?;
    cx.apply_function(default, Vec::new())
}

fn eval_and_then(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = named_or_first(arguments, "by")
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`and_then` needs a function"))?;
    let result = super::collections::produced(by).present().clone();
    match held(&input) {
        Some(value) => Ok(match cx.apply_lambda(by, value)? {
            Value::Present(inner) => Value::Present(inner),
            Value::Absent(_) => Value::Absent(result),
            other => Value::Present(Arc::new(other)),
        }),
        None => Ok(Value::Absent(result)),
    }
}

fn eval_or_else(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if held(&input).is_some() {
        return Ok(input);
    }
    let default = named_or_first(arguments, "default")
        .ok_or_else(|| Diagnostic::runtime(span, "`or_else` needs a function"))?;
    cx.apply_function(default, Vec::new())
}

fn eval_alternative(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if held(&input).is_some() {
        return Ok(input);
    }
    let other = arguments
        .first()
        .ok_or_else(|| Diagnostic::runtime(span, "`alternative` needs another option"))?;
    cx.evaluate(&other.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_has_a_coherent_spec() {
        for step in STEPS {
            step.spec()
                .validate()
                .unwrap_or_else(|error| panic!("{}: {error}", step.name));
        }
    }
}
