//! Checking and evaluating explicit construction of materialized collections.

use crate::ast::Arg;
use crate::diagnostics::Diagnostic;
use crate::extensions::{CheckCx, EvalCx};
use crate::types::{MapKind, MapValue, TypeConstructor, TypeRef, Value, builtin, collections};
use std::ops::Range;
use std::rc::Rc;

pub(crate) fn named(name: &str) -> Option<TypeConstructor> {
    [
        collections::COLLECTION,
        collections::SEQ,
        collections::LIST,
        collections::SET,
        collections::MAP,
        collections::ORDERED_MAP,
        collections::SORTED_MAP,
        collections::ORDERABLE,
    ]
    .into_iter()
    .find(|constructor| constructor.0 == name)
}

pub(crate) fn check(
    cx: &mut dyn CheckCx,
    constructor: TypeConstructor,
    args: &[Arg],
    expected: Option<&TypeRef>,
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let system =
        builtin::system().map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
    let definition = system
        .definition(constructor)
        .map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
    if definition.kind != crate::types::TypeKind::Concrete {
        return Err(Diagnostic::typing(
            span,
            format!("{constructor} is abstract; construct a concrete subtype"),
        ));
    }
    let arity = if MapKind::of(constructor).is_some() {
        2
    } else {
        1
    };
    if args.len() != arity || args.iter().any(|arg| arg.name.is_some()) {
        return Err(Diagnostic::typing(
            span,
            format!("{constructor} expects {arity} positional collection arguments"),
        ));
    }
    let mut elements = Vec::new();
    for argument in args {
        let input = cx.infer(&argument.value)?;
        if arity == 2 && !input.is_sequence() {
            return Err(Diagnostic::typing(
                argument.value.span.clone(),
                format!("map keys and values must be sequences, not {input}"),
            ));
        }
        elements.push(input.element().ok_or_else(|| {
            Diagnostic::typing(
                argument.value.span.clone(),
                format!("{constructor} needs a collection, not {input}"),
            )
        })?);
    }
    if let Some(expected) = expected {
        for (actual, wanted) in elements.iter().zip(&expected.args) {
            if !actual.is(wanted) {
                return Err(Diagnostic::typing(
                    span.clone(),
                    format!("{actual} does not inhabit constructor argument {wanted}"),
                ));
            }
        }
        system
            .validate_concrete(expected)
            .map_err(|error| Diagnostic::typing(span, error.to_string()))?;
        return Ok(expected.clone());
    }
    system
        .apply(constructor, elements)
        .map_err(|error| Diagnostic::typing(span, error.to_string()))
}

pub(crate) fn evaluate(
    cx: &mut dyn EvalCx,
    constructor: TypeConstructor,
    args: &[Arg],
    expected: Option<&TypeRef>,
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let mut inputs = Vec::new();
    for argument in args {
        inputs.push(cx.evaluate(&argument.value)?);
    }
    let element = |index: usize| {
        expected
            .and_then(|reference| reference.args.get(index))
            .cloned()
            .or_else(|| {
                inputs
                    .get(index)
                    .and_then(|input| input.type_of().element())
            })
            .unwrap_or(TypeRef::NEVER)
    };
    let elements = |index: usize| {
        inputs.get(index).and_then(Value::elements).ok_or_else(|| {
            Diagnostic::runtime(
                span.clone(),
                format!("{constructor} needs collection arguments"),
            )
        })
    };
    if let Some(kind) = MapKind::of(constructor) {
        let map = MapValue::new(kind, elements(0)?, elements(1)?, element(0), element(1))
            .map_err(|error| Diagnostic::runtime(span, error.to_string()))?;
        return Ok(Value::Map(Rc::new(map)));
    }
    match constructor {
        collections::LIST => Ok(Value::list(elements(0)?, element(0))),
        collections::SET => Ok(Value::set(elements(0)?, element(0))),
        _ => Err(Diagnostic::runtime(
            span,
            format!("{constructor} has no runtime constructor"),
        )),
    }
}
