//! The `bytes` module (HQL-0053 part 1): the natives behind
//! `lib/std/bytes/mod.hql`, and the `Bytes` constructors of `core` and
//! `encoding`.
//!
//! A `Bytes` value is a finite sequence of octets. Its positions are its
//! octets, each read as an `Int` in `0..255`.

use super::{EvalCx, Operation, declared};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{TypeRef, Value};
use std::ops::Range;
use std::sync::Arc;

/// `Bytes(text)` of `core`, and `Bytes(text, as)` of `encoding`: one
/// native, which reads an encoding where the call names one.
pub(crate) const BYTES: Operation = declared("Bytes", eval_bytes);

/// Every step the `bytes` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    declared("at", eval_at),
    declared("slice", eval_slice),
    declared("size", eval_size),
    declared("to_text", eval_to_text),
];

/// The octets a `Bytes` input holds.
pub(crate) fn input_octets(
    input: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<Arc<[u8]>, Diagnostic> {
    match input {
        Value::Bytes(octets) => Ok(Arc::clone(octets)),
        Value::Present(inner) => input_octets(inner, name, span),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` needs Bytes, not {}", other.type_of()),
        )),
    }
}

/// The text a `String` input holds.
pub(crate) fn input_text(
    input: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<String, Diagnostic> {
    match input {
        Value::Str(text) => Ok(text.to_string()),
        Value::Formatted(formatted) => Ok(formatted.text.clone()),
        Value::Present(inner) => input_text(inner, name, span),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` needs a String, not {}", other.type_of()),
        )),
    }
}

/// The value of an `Int` argument.
fn int_argument(cx: &mut dyn EvalCx, argument: &TypedArg, name: &str) -> Result<i64, Diagnostic> {
    match cx.evaluate(&argument.value)? {
        Value::Int(value) => Ok(value),
        other => Err(Diagnostic::runtime(
            argument.value.span.clone(),
            format!("`{name}` needs an Int, not {}", other.type_of()),
        )),
    }
}

fn eval_bytes(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    // The text is the call's input where it was piped, else its first
    // argument written without a name.
    let mut positional = arguments.iter().filter(|argument| argument.name.is_none());
    let text = match input {
        Value::Unit => {
            let argument = positional.next().ok_or_else(|| {
                Diagnostic::runtime(span.clone(), "`Bytes` needs a text: `Bytes(\"hello\")`")
            })?;
            let value = cx.evaluate(&argument.value)?;
            input_text(&value, "Bytes", &argument.value.span)?
        }
        other => input_text(&other, "Bytes", &span)?,
    };
    let encoding = super::named(arguments, "as").or_else(|| positional.next());
    let Some(encoding) = encoding else {
        return Ok(Value::Bytes(text.into_bytes().into()));
    };
    let encoding = super::encoding::Encoding::of(encoding).ok_or_else(|| {
        Diagnostic::runtime(
            span.clone(),
            "`Bytes` needs an encoding: `Hex`, `Base64`, ...",
        )
    })?;
    encoding
        .decode_at(&text)
        .map(|octets| Value::Bytes(octets.into()))
        .map_err(|position| {
            Diagnostic::runtime(
                span.clone(),
                format!(
                    "the text is not valid {}: the first invalid character is at position {position}",
                    encoding.name()
                ),
            )
        })
}

fn eval_at(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let octets = input_octets(&input, "at", &span)?;
    let argument = arguments
        .first()
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`at` needs a position"))?;
    let position = int_argument(cx, argument, "at")?;
    crate::evaluation::at_position(Value::Bytes(octets), position, &TypeRef::INT, &span)
}

fn eval_slice(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let octets = Value::Bytes(input_octets(&input, "slice", &span)?);
    if let [range] = arguments {
        let range = cx.evaluate(&range.value)?;
        let (start, end) = crate::evaluation::range_bounds(&range).ok_or_else(|| {
            Diagnostic::runtime(span.clone(), "`slice` with one argument takes a Range")
        })?;
        return crate::evaluation::slice_of(octets, start, end, &span);
    }
    let [start, end] = arguments else {
        return Err(Diagnostic::runtime(
            span,
            "`slice` needs a start and an end",
        ));
    };
    let start = int_argument(cx, start, "slice")?;
    let end = int_argument(cx, end, "slice")?;
    crate::evaluation::slice_of(octets, Some(start), Some(end), &span)
}

fn eval_size(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let octets = input_octets(&input, "size", &span)?;
    Ok(Value::Int(i64::try_from(octets.len()).unwrap_or(i64::MAX)))
}

fn eval_to_text(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let octets = input_octets(&input, "to_text", &span)?;
    Ok(match std::str::from_utf8(&octets) {
        Ok(text) => Value::Present(Arc::new(Value::Str(Arc::from(text)))),
        Err(_) => Value::Absent(TypeRef::STR),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_has_a_coherent_spec() {
        for step in STEPS.iter().chain([&BYTES]) {
            step.spec()
                .validate()
                .unwrap_or_else(|error| panic!("{}: {error}", step.name));
        }
    }
}
