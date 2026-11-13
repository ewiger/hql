//! The `result` module: the ways out of a `Result<T, E>`, each saying what
//! a failure means. There is no unwrap.

use super::declared;
use super::{EvalCx, Operation, named_or_first};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{TypeRef, Value};
use std::ops::Range;
use std::sync::Arc;

/// Every step the `result` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    declared("is_ok", eval_is_ok),
    declared("is_err", eval_is_err),
    declared("ok", eval_ok),
    declared("err", eval_err),
    declared("unwrap_or", eval_unwrap_or),
    declared("unwrap_or_else", eval_unwrap_or_else),
    declared("map_err", eval_map_err),
    declared("and_then", eval_and_then),
    declared("or_else", eval_or_else),
];

fn parts(input: &Value, name: &str, span: &Range<usize>) -> Result<Value, Diagnostic> {
    match input {
        Value::Ok(..) | Value::Err(..) => Ok(input.clone()),
        Value::Present(inner) => parts(inner, name, span),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` needs a Result, not {}", other.type_of()),
        )),
    }
}

fn eval_is_ok(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Bool(matches!(
        parts(&input, "is_ok", &span)?,
        Value::Ok(..)
    )))
}

fn eval_is_err(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Bool(matches!(
        parts(&input, "is_err", &span)?,
        Value::Err(..)
    )))
}

fn eval_ok(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(match parts(&input, "ok", &span)? {
        Value::Ok(value, _) => Value::Present(value),
        Value::Err(_, value) => Value::Absent(value),
        _ => unreachable!("parts() answers a result"),
    })
}

fn eval_err(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(match parts(&input, "err", &span)? {
        Value::Err(error, _) => Value::Present(error),
        Value::Ok(_, error) => Value::Absent(error),
        _ => unreachable!("parts() answers a result"),
    })
}

fn eval_unwrap_or(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    match parts(&input, "unwrap_or", &span)? {
        Value::Ok(value, _) => Ok(value.as_ref().clone()),
        _ => {
            let default = arguments
                .first()
                .ok_or_else(|| Diagnostic::runtime(span, "`unwrap_or` needs a default"))?;
            cx.evaluate(&default.value)
        }
    }
}

fn eval_unwrap_or_else(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    match parts(&input, "unwrap_or_else", &span)? {
        Value::Ok(value, _) => Ok(value.as_ref().clone()),
        Value::Err(error, _) => {
            let default = named_or_first(arguments, "default")
                .ok_or_else(|| Diagnostic::runtime(span, "`unwrap_or_else` needs a function"))?;
            cx.apply_lambda(default, error.as_ref().clone())
        }
        _ => unreachable!("parts() answers a result"),
    }
}

fn eval_map_err(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = named_or_first(arguments, "by")
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`map_err` needs a function"))?;
    let produced = super::collections::produced(by);
    match parts(&input, "map_err", &span)? {
        Value::Ok(value, _) => Ok(Value::Ok(value, produced)),
        Value::Err(error, value) => Ok(Value::Err(
            Arc::new(cx.apply_lambda(by, error.as_ref().clone())?),
            value,
        )),
        _ => unreachable!("parts() answers a result"),
    }
}

fn eval_and_then(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = named_or_first(arguments, "by")
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`and_then` needs a function"))?;
    let produced = super::collections::produced(by);
    let value_type = produced
        .as_result()
        .map_or(TypeRef::NEVER, |(value, _)| value.clone());
    match parts(&input, "and_then", &span)? {
        Value::Ok(value, _) => cx.apply_lambda(by, value.as_ref().clone()),
        Value::Err(error, _) => Ok(Value::Err(error, value_type)),
        _ => unreachable!("parts() answers a result"),
    }
}

fn eval_or_else(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = named_or_first(arguments, "by")
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`or_else` needs a function"))?;
    let produced = super::collections::produced(by);
    let error_type = produced
        .as_result()
        .map_or(TypeRef::NEVER, |(_, error)| error.clone());
    match parts(&input, "or_else", &span)? {
        Value::Ok(value, _) => Ok(Value::Ok(value, error_type)),
        Value::Err(error, _) => cx.apply_lambda(by, error.as_ref().clone()),
        _ => unreachable!("parts() answers a result"),
    }
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
