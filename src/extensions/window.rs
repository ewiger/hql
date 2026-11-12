//! Analytic windows (HQL-0050): `over` and the functions of a `Window<T>`,
//! one row's place in its ordered partition. The `seq` module declares and
//! registers them; a window's size is the `collections` module's `size`.
//!
//! A window is a record of the window type whose fields no program can name:
//! the partition in its order, shared by every window of the partition, and
//! the row's position and ranks within it.

use super::collections::{arity_of, elements, produced, wrong};
use super::{EvalCx, named};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::builtin::WINDOW;
use crate::types::{Record, TypeRef, Value};
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

const ROWS: &str = "rows";
const AT: &str = "at";
const RANK: &str = "rank";
const DENSE_RANK: &str = "dense_rank";

/// A window's parts, read back from its record.
pub(crate) struct View {
    rows: Arc<Vec<Value>>,
    element: TypeRef,
    at: usize,
    rank: i64,
    dense_rank: i64,
}

/// The parts of `value` where it is a window.
pub(crate) fn view(value: &Value) -> Option<View> {
    let Value::Record(record) = value else {
        return None;
    };
    if record.ty.constructor != WINDOW {
        return None;
    }
    let field = |name: &str| {
        record
            .fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    };
    let Some(Value::List(rows, element)) = field(ROWS) else {
        return None;
    };
    let int = |name: &str| match field(name) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    };
    Some(View {
        rows: Arc::clone(rows),
        element: element.clone(),
        at: usize::try_from(int(AT)?).ok()?,
        rank: int(RANK)?,
        dense_rank: int(DENSE_RANK)?,
    })
}

fn window(rows: &Value, element: &TypeRef, at: usize, rank: i64, dense_rank: i64) -> Value {
    let int = |n: usize| Value::Int(i64::try_from(n).unwrap_or(i64::MAX));
    Value::Record(Arc::new(Record {
        ty: WINDOW.apply([element.clone()]),
        fields: vec![
            (ROWS.to_owned(), rows.clone()),
            (AT.to_owned(), int(at)),
            (RANK.to_owned(), Value::Int(rank)),
            (DENSE_RANK.to_owned(), Value::Int(dense_rank)),
        ],
        carrier: None,
    }))
}

fn int(n: usize, span: &Range<usize>) -> Result<Value, Diagnostic> {
    i64::try_from(n)
        .map(Value::Int)
        .map_err(|_| Diagnostic::runtime(span.clone(), "a position exceeds Int"))
}

fn of(input: &Value, name: &str, span: &Range<usize>) -> Result<View, Diagnostic> {
    view(input).ok_or_else(|| wrong(name, "a window", span))
}

/// How two rows compare under the order `over` was given: a comparison of
/// two rows, or a key of each, compared as values.
enum Order<'a> {
    Comparison(&'a TypedArg),
    Keys(Vec<Value>),
}

impl Order<'_> {
    fn between(
        &self,
        cx: &mut dyn EvalCx,
        rows: &[Value],
        left: usize,
        right: usize,
        span: &Range<usize>,
    ) -> Result<Ordering, Diagnostic> {
        match self {
            Self::Comparison(by) => {
                match cx.apply_comparator(by, rows[left].clone(), rows[right].clone())? {
                    Value::Ordering(order) => Ok(order),
                    _ => Err(Diagnostic::runtime(
                        span.clone(),
                        "an `over` order must return Ordering",
                    )),
                }
            }
            Self::Keys(keys) => cx.compare_values(&keys[left], &keys[right], span),
        }
    }
}

pub(crate) fn eval_over(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let order = named(arguments, "order").ok_or_else(|| wrong("over", "an `order`", &span))?;
    let into = named(arguments, "into").ok_or_else(|| wrong("over", "an `into`", &span))?;
    let by = named(arguments, "by");
    let element = super::collections::element_type(&input);
    let rows = elements(&input, "over", &span)?;

    // Partitions in the order their keys were first seen, each holding the
    // input positions of its rows.
    let mut partitions: Vec<(Value, Vec<usize>)> = Vec::new();
    for (position, row) in rows.iter().enumerate() {
        let key = match by {
            Some(by) => cx.apply_lambda(by, row.clone())?,
            None => Value::Unit,
        };
        match partitions
            .iter_mut()
            .find(|(seen, _)| crate::evaluation::same_key(seen, &key))
        {
            Some((_, positions)) => positions.push(position),
            None => partitions.push((key, vec![position])),
        }
    }

    let order = if arity_of(order) == Some(2) {
        Order::Comparison(order)
    } else {
        let mut keys = Vec::with_capacity(rows.len());
        for row in &rows {
            keys.push(cx.apply_lambda(order, row.clone())?);
        }
        Order::Keys(keys)
    };

    let mut results: Vec<Option<Value>> = vec![None; rows.len()];
    for (_, mut positions) in partitions {
        // A stable insertion sort: rows equal under the order keep their
        // input positions, and a failing comparison is never hidden.
        for index in 1..positions.len() {
            let mut at = index;
            while at > 0
                && order.between(cx, &rows, positions[at - 1], positions[at], &span)?
                    == Ordering::Greater
            {
                positions.swap(at - 1, at);
                at -= 1;
            }
        }
        let ordered = Value::list(
            positions
                .iter()
                .map(|&position| rows[position].clone())
                .collect(),
            element.clone(),
        );
        let (mut rank, mut dense_rank) = (1_i64, 1_i64);
        for (at, &position) in positions.iter().enumerate() {
            if at > 0
                && order.between(cx, &rows, positions[at - 1], position, &span)? != Ordering::Equal
            {
                rank = i64::try_from(at + 1).unwrap_or(i64::MAX);
                dense_rank += 1;
            }
            let window = window(&ordered, &element, at, rank, dense_rank);
            results[position] = Some(cx.apply_comparator(into, rows[position].clone(), window)?);
        }
    }
    Ok(Value::list(
        results.into_iter().flatten().collect(),
        produced(into),
    ))
}

pub(crate) fn eval_row_number(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let window = of(&input, "row_number", &span)?;
    int(window.at + 1, &span)
}

pub(crate) fn eval_rank(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Int(of(&input, "rank", &span)?.rank))
}

pub(crate) fn eval_dense_rank(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Int(of(&input, "dense_rank", &span)?.dense_rank))
}

/// A count argument: `n` by name or first, `default` where absent, and an
/// evaluation error below 0.
fn count(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    name: &str,
    parameter: &str,
    default: Option<usize>,
    span: &Range<usize>,
) -> Result<usize, Diagnostic> {
    let argument = match named(arguments, parameter).or_else(|| {
        arguments
            .iter()
            .filter(|argument| argument.name.is_none())
            .nth(usize::from(parameter == "after"))
    }) {
        Some(argument) => argument,
        None => {
            return default.ok_or_else(|| wrong(name, &format!("`{parameter}`"), span));
        }
    };
    let Value::Int(n) = cx.evaluate(&argument.value)? else {
        return Err(wrong(name, &format!("an Int `{parameter}`"), span));
    };
    usize::try_from(n).map_err(|_| {
        Diagnostic::runtime(
            span.clone(),
            format!("`{name}` takes `{parameter}` of 0 or more, not {n}"),
        )
    })
}

fn neighbour(window: &View, offset: Option<usize>) -> Value {
    match offset.and_then(|at| window.rows.get(at)) {
        Some(row) => Value::Present(Arc::new(row.clone())),
        None => Value::Absent(window.element.clone()),
    }
}

pub(crate) fn eval_lag(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let window = of(&input, "lag", &span)?;
    let n = count(cx, arguments, "lag", "n", Some(1), &span)?;
    Ok(neighbour(&window, window.at.checked_sub(n)))
}

pub(crate) fn eval_lead(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let window = of(&input, "lead", &span)?;
    let n = count(cx, arguments, "lead", "n", Some(1), &span)?;
    Ok(neighbour(&window, window.at.checked_add(n)))
}

fn frame(window: &View, range: Range<usize>) -> Value {
    Value::list(window.rows[range].to_vec(), window.element.clone())
}

pub(crate) fn eval_running(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let window = of(&input, "running", &span)?;
    Ok(frame(&window, 0..window.at + 1))
}

pub(crate) fn eval_rows(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let window = of(&input, "rows", &span)?;
    let before = count(cx, arguments, "rows", "before", None, &span)?;
    let after = count(cx, arguments, "rows", "after", None, &span)?;
    let start = window.at.saturating_sub(before);
    let end = window
        .at
        .saturating_add(after)
        .saturating_add(1)
        .min(window.rows.len());
    Ok(frame(&window, start..end))
}

pub(crate) fn eval_whole(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let window = of(&input, "whole", &span)?;
    Ok(frame(&window, 0..window.rows.len()))
}

/// The first row of the window's partition, where `input` is a window.
pub(crate) fn first(input: &Value) -> Option<Value> {
    view(input).and_then(|window| window.rows.first().cloned())
}

/// The last row of the window's partition, where `input` is a window.
pub(crate) fn last(input: &Value) -> Option<Value> {
    view(input).and_then(|window| window.rows.last().cloned())
}

/// The number of rows in the window's partition, where `input` is a window.
pub(crate) fn size(input: &Value) -> Option<usize> {
    view(input).map(|window| window.rows.len())
}

/// Whether `ty` holds a window anywhere: `into` may not give one, since a
/// window is a view of its partition during the call (HQL-0050).
pub(crate) fn mentions_window(ty: &TypeRef) -> bool {
    ty.constructor == WINDOW || ty.args.iter().any(mentions_window)
}

/// `over` takes a sequence, and its `into` gives no window.
pub(crate) fn check_over(
    _: &mut dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if let Some(result) = call.bindings.get("U")
        && mentions_window(result)
    {
        return Err(Diagnostic::typing(
            span,
            format!(
                "`over`'s `into` gives {result}: a window is a view of its partition during the call and does not leave it"
            ),
        ));
    }
    Ok(())
}
