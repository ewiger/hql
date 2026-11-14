//! The `random` module (HQL-0041 part 4): the natives behind every
//! declaration of `lib/std/random/mod.hql`.
//!
//! A call with a `seed` argument draws from a generator of that seed and its
//! call site, made afresh for the call, so it is a function of its input. A
//! call without one draws from its call site's generator of the evaluation
//! seed, which carries on from one call to the next. The generator and the
//! seed are [`crate::random`]'s.

use super::collections::{element_type, elements, in_element_order, pending};
use super::{EvalCx, Operation, Purity, Shape, Signature};
use crate::diagnostics::{Diagnostic, Disposition, Kind, Phase, Severity};
use crate::execution::Arg as TypedArg;
use crate::random::{self as r, Generator, Reservoir};
use crate::types::Value;
use std::ops::Range;
use std::sync::Arc;

/// An operation whose unseeded forms draw from the evaluation seed: the
/// marking of a name whose forms differ in purity is effectful.
const fn draw(name: &'static str, eval: super::Eval) -> Operation {
    Operation {
        name,
        spec: Signature::Declared,
        purity: Purity::Effectful,
        shape: Shape::Barrier,
        check: None,
        eval,
    }
}

/// Every step the `random` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    draw("sample", eval_sample),
    draw("shuffle", eval_shuffle),
    draw("choice", eval_choice),
    draw("random_int", eval_random_int),
    draw("random_float", eval_random_float),
    draw("uuid", eval_uuid),
];

type Result<T> = std::result::Result<T, Diagnostic>;

fn fail(span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
    Diagnostic::runtime(span.clone(), message)
}

/// The call site a draw is keyed by: where the call is written.
fn site(span: &Range<usize>) -> u64 {
    span.start as u64
}

/// Where a call draws from: a generator of its own seed, or its call site's
/// generator of the evaluation seed.
enum Source {
    Seeded(Box<Generator>),
    Evaluation(u64),
}

impl Source {
    /// The source of a call: `seed` where it is written.
    fn of(cx: &mut dyn EvalCx, arguments: &[TypedArg], span: &Range<usize>) -> Result<Self> {
        match super::named(arguments, "seed") {
            Some(argument) => match cx.evaluate(&argument.value)? {
                Value::Int(seed) => Ok(Self::Seeded(Box::new(Generator::new(seed, site(span), 0)))),
                _ => Err(fail(span, "`seed` needs an Int")),
            },
            None => Ok(Self::Evaluation(site(span))),
        }
    }

    /// Run `use_generator` on this source's generator.
    fn with<R>(self, use_generator: impl FnOnce(&mut Generator) -> R) -> R {
        match self {
            Self::Seeded(mut generator) => use_generator(&mut generator),
            Self::Evaluation(at) => {
                let (drawn, reported) = r::with_evaluation_stream(at, use_generator);
                report(reported);
                drawn
            }
        }
    }
}

/// 64 bits drawn at a call site from the evaluation seed: what a step
/// without a `seed` of its own draws from.
pub(crate) fn draw_bits(span: &Range<usize>) -> u64 {
    Source::Evaluation(site(span)).with(Generator::next_u64)
}

/// Report a seed this evaluation drew from entropy, so `--seed` repeats it.
fn report(seed: Option<i64>) {
    if let Some(seed) = seed {
        crate::io::write_diagnostic(Diagnostic::new(
            Phase::Execute,
            Kind::Effect,
            Severity::Info,
            Disposition::Warning,
            0..0,
            format!("the evaluation seed is {seed}; `--seed {seed}` repeats this evaluation"),
        ));
    }
}

/// The count `n` of a sample: the first argument written without a name.
fn count(cx: &mut dyn EvalCx, arguments: &[TypedArg], span: &Range<usize>) -> Result<usize> {
    let argument = super::named_or_first(arguments, "n")
        .filter(|argument| argument.name.as_deref() != Some("seed"))
        .ok_or_else(|| fail(span, "`sample` needs a count: `sample(10)`"))?;
    match cx.evaluate(&argument.value)? {
        Value::Int(n) => usize::try_from(n).map_err(|_| {
            fail(
                &argument.value.span,
                format!("`sample` needs a count of zero or more, and {n} is below zero"),
            )
        }),
        _ => Err(fail(span, "`sample` needs an Int count")),
    }
}

/// The elements of a collection in the order a draw reads them: a
/// sequence's own, and an unordered collection's sorted by its elements.
fn ordered(
    cx: &mut dyn EvalCx,
    input: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<Vec<Value>> {
    let mut held = elements(input, name, span)?;
    if matches!(input, Value::Set(..)) {
        in_element_order(cx, &mut held, span)?;
    }
    Ok(held)
}

fn eval_sample(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let n = count(cx, arguments, &span)?;
    let source = Source::of(cx, arguments, &span)?;
    // A stream is sampled as it passes, holding `n` elements.
    if let Some(traversal) = pending(&input)
        && !traversal.finite
    {
        let element = traversal.element.clone();
        let mut reservoir = Reservoir::new(n);
        if n > 0 {
            // The stream's elements may themselves draw, so the call site's
            // generator is taken out while it passes and put back after.
            let (mut generator, at) = match source {
                Source::Seeded(generator) => (*generator, None),
                Source::Evaluation(at) => {
                    let (generator, reported) = r::with_evaluation_stream(at, |held| held.clone());
                    report(reported);
                    (generator, Some(at))
                }
            };
            cx.traverse(traversal, &mut |_, value| {
                reservoir.offer(&mut generator, value);
                Ok(true)
            })?;
            if let Some(at) = at {
                r::with_evaluation_stream(at, |held| *held = generator);
            }
        }
        return Ok(Value::list(reservoir.into_sample(), element));
    }
    let element = element_type(&input);
    let held = ordered(cx, &input, "sample", &span)?;
    let size = held.len();
    let positions = source.with(|generator| generator.positions(size, n));
    let sample = positions.into_iter().map(|at| held[at].clone()).collect();
    Ok(Value::list(sample, element))
}

fn eval_shuffle(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let source = Source::of(cx, arguments, &span)?;
    let element = element_type(&input);
    let mut held = ordered(cx, &input, "shuffle", &span)?;
    source.with(|generator| generator.shuffle(&mut held));
    Ok(Value::list(held, element))
}

fn eval_choice(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let source = Source::of(cx, arguments, &span)?;
    let element = element_type(&input);
    let held = ordered(cx, &input, "choice", &span)?;
    if held.is_empty() {
        return Ok(Value::Absent(element));
    }
    let size = held.len();
    let at = source.with(|generator| generator.index(size));
    Ok(Value::Present(Arc::new(held[at].clone())))
}

fn eval_random_int(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let high = super::named_or_first(arguments, "high")
        .ok_or_else(|| fail(&span, "`random_int` needs a low and a high bound"))?;
    let (Value::Int(low), Value::Int(high)) = (input, cx.evaluate(&high.value)?) else {
        return Err(fail(&span, "`random_int` needs Int bounds"));
    };
    if high < low {
        return Err(fail(
            &span,
            format!("`random_int` needs `high` at or above `low`, and {high} is below {low}"),
        ));
    }
    let source = Source::Evaluation(site(&span));
    Ok(Value::Int(
        source.with(|generator| generator.between(low, high)),
    ))
}

fn eval_random_float(
    _: &mut dyn EvalCx,
    _: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let source = Source::Evaluation(site(&span));
    Ok(Value::Float(source.with(Generator::unit)))
}

fn eval_uuid(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let Value::Str(name) = input else {
        return version_7(&span);
    };
    let namespace = super::named_or_first(arguments, "namespace")
        .ok_or_else(|| fail(&span, "`uuid` needs a namespace"))?;
    let Value::Str(namespace) = cx.evaluate(&namespace.value)? else {
        return Err(fail(&span, "`uuid` needs a String namespace"));
    };
    let space = match namespace.as_ref() {
        "dns" => uuid::Uuid::NAMESPACE_DNS,
        "url" => uuid::Uuid::NAMESPACE_URL,
        "oid" => uuid::Uuid::NAMESPACE_OID,
        "x500" => uuid::Uuid::NAMESPACE_X500,
        text => uuid::Uuid::parse_str(text).map_err(|_| {
            fail(
                &span,
                format!(
                    "`uuid` needs a namespace that is a UUID or one of \"dns\", \"url\", \"oid\" \
                     and \"x500\", and {text:?} is neither"
                ),
            )
        })?,
    };
    Ok(text(uuid::Uuid::new_v5(&space, name.as_bytes())))
}

/// A version 7 UUID: 48 bits of the evaluation instant's Unix
/// milliseconds, the version, 12 bits of the counter, the variant, its
/// other 30 bits, and 32 bits drawn at the call site.
fn version_7(span: &Range<usize>) -> Result<Value> {
    let (now, _, _) = crate::time::evaluation_clock().map_err(|message| fail(span, message))?;
    let millis = u128::try_from(now.as_millisecond()).unwrap_or(0) & ((1 << 48) - 1);
    let (counter, reported) = r::next_counter();
    report(reported);
    let counter = u128::from(counter);
    let drawn =
        u128::from(Source::Evaluation(site(span)).with(|generator| generator.next_u64() >> 32));
    let bits = (millis << 80)
        | (0x7 << 76)
        | ((counter >> 30) << 64)
        | (0b10 << 62)
        | ((counter & 0x3FFF_FFFF) << 32)
        | drawn;
    Ok(text(uuid::Uuid::from_u128(bits)))
}

fn text(id: uuid::Uuid) -> Value {
    Value::Str(Arc::from(id.hyphenated().to_string()))
}
