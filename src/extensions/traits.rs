//! The native implementations of the core traits: the function bodies behind
//! the bodyless `impl` headers of `lib/std/core/mod.hql`.
//!
//! A header is what says a built-in type implements a trait; this table only
//! supplies what runs. Binding refuses an entry without a header and a header
//! without an entry, so the runtime cannot add an implementation the language
//! has not declared.

use crate::ast::Arith;
use crate::diagnostics::Diagnostic;
use crate::types::{TypeRef, Value};
use std::ops::Range;

/// The functions of one trait, run natively over a receiver and the
/// arguments after it.
type Body = fn(&str, Value, Vec<Value>, &Range<usize>) -> Result<Value, Diagnostic>;

/// One native implementation: the trait, the implementing type's name and
/// the body of every function the trait requires.
#[derive(Debug)]
pub(crate) struct NativeImpl {
    pub trait_name: &'static str,
    pub target: &'static str,
    pub body: Body,
}

const fn native(trait_name: &'static str, target: &'static str, body: Body) -> NativeImpl {
    NativeImpl {
        trait_name,
        target,
        body,
    }
}

/// Every native implementation the binary carries.
pub(crate) static NATIVE_IMPLS: &[NativeImpl] = &[
    native("Eq", "Bool", eq),
    native("Eq", "Int", eq),
    native("Eq", "Float", eq),
    native("Eq", "Decimal", eq),
    native("Eq", "String", eq),
    native("Eq", "Date", eq),
    native("Ord", "Int", compare),
    native("Ord", "Float", compare),
    native("Ord", "Decimal", compare),
    native("Ord", "String", compare),
    native("Ord", "Date", compare),
    native("Add", "Int", arithmetic),
    native("Add", "Float", arithmetic),
    native("Add", "Decimal", arithmetic),
    native("Add", "String", concatenation),
    native("Sub", "Int", arithmetic),
    native("Sub", "Float", arithmetic),
    native("Sub", "Decimal", arithmetic),
    native("Mul", "Int", arithmetic),
    native("Mul", "Float", arithmetic),
    native("Mul", "Decimal", arithmetic),
    native("Div", "Int", arithmetic),
    native("Div", "Float", arithmetic),
    native("Div", "Decimal", arithmetic),
    native("Rem", "Int", arithmetic),
    native("Rem", "Float", arithmetic),
    native("Rem", "Decimal", arithmetic),
    native("Index", "Seq", position),
    native("Index", "String", position),
    native("Index", "Data", position),
    native("Slice", "Seq", part),
    native("Slice", "String", part),
    native("Slice", "Data", part),
];

/// The native implementation of a trait for a type, as the process's
/// environment bound it: the body behind the header declared for the type
/// itself, or for a type it narrows, as a List takes Seq's.
pub(crate) fn native_impl(trait_name: &str, receiver: &TypeRef) -> Option<&'static NativeImpl> {
    super::binding::Environment::get()
        .ok()?
        .native_body(trait_name, receiver)
}

/// The native implementation a header declares, by its trait and its
/// implementing type's name.
pub(crate) fn native_header_impl(trait_name: &str, target: &str) -> Option<&'static NativeImpl> {
    NATIVE_IMPLS
        .iter()
        .find(|held| held.trait_name == trait_name && held.target == target)
}

fn other(method: &str, arguments: Vec<Value>, span: &Range<usize>) -> Result<Value, Diagnostic> {
    arguments
        .into_iter()
        .next()
        .ok_or_else(|| Diagnostic::runtime(span.clone(), format!("`{method}` takes `other`")))
}

fn eq(
    method: &str,
    receiver: Value,
    arguments: Vec<Value>,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Bool(receiver == other(method, arguments, span)?))
}

fn compare(
    method: &str,
    receiver: Value,
    arguments: Vec<Value>,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let other = other(method, arguments, span)?;
    let order = receiver
        .order_key()
        .zip(other.order_key())
        .map(|(left, right)| left.compare(&right))
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`compare` needs two ordered values"))?;
    Ok(Value::Ordering(order))
}

fn arithmetic(
    method: &str,
    receiver: Value,
    arguments: Vec<Value>,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let operator = match method {
        "add" => Arith::Add,
        "sub" => Arith::Sub,
        "mul" => Arith::Mul,
        "div" => Arith::Div,
        "rem" => Arith::Rem,
        _ => {
            return Err(Diagnostic::runtime(
                span.clone(),
                format!("no arithmetic trait has `{method}`"),
            ));
        }
    };
    crate::evaluation::arithmetic_of(operator, receiver, other(method, arguments, span)?, span)
}

fn concatenation(
    method: &str,
    receiver: Value,
    arguments: Vec<Value>,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    match (receiver, other(method, arguments, span)?) {
        (Value::Str(left), Value::Str(right)) => Ok(Value::Str(format!("{left}{right}").into())),
        _ => Err(Diagnostic::runtime(
            span.clone(),
            "`add` over String takes two Strings",
        )),
    }
}

fn position(
    method: &str,
    receiver: Value,
    arguments: Vec<Value>,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let Some(Value::Int(position)) = arguments.into_iter().next() else {
        return Err(Diagnostic::runtime(
            span.clone(),
            format!("`{method}` takes a position, an Int"),
        ));
    };
    crate::evaluation::at_position(receiver, position, &TypeRef::NEVER, span)
}

fn part(
    method: &str,
    receiver: Value,
    arguments: Vec<Value>,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let Some((start, end)) = arguments.first().and_then(crate::evaluation::range_bounds) else {
        return Err(Diagnostic::runtime(
            span.clone(),
            format!("`{method}` takes a Range"),
        ));
    };
    crate::evaluation::slice_of(receiver, start, end, span)
}
