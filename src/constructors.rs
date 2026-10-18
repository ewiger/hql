//! Checking and evaluating explicit construction of materialized collections,
//! and of a graph projected from cards.

use crate::ast::Arg;
use crate::diagnostics::Diagnostic;
use crate::extensions::{CheckCx, EvalCx};
use crate::types::{MapKind, MapValue, TypeConstructor, TypeRef, Value, builtin, collections};
use std::ops::Range;
use std::sync::Arc;

/// The types a program may name as constructors, abstract ones included so
/// that naming one is refused with its concrete subtypes.
const CONSTRUCTORS: [TypeConstructor; 9] = [
    collections::COLLECTION,
    collections::SEQ,
    collections::LIST,
    collections::SET,
    collections::MAP,
    collections::ORDERED_MAP,
    collections::SORTED_MAP,
    collections::ORDERABLE,
    GRAPH,
];

const GRAPH: TypeConstructor = TypeConstructor("Graph");

pub(crate) fn named(name: &str) -> Option<TypeConstructor> {
    CONSTRUCTORS
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
    if constructor == GRAPH {
        return check_graph(cx, args, span);
    }
    let system =
        builtin::system().map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
    let definition = system
        .definition(constructor)
        .map_err(|error| Diagnostic::typing(span.clone(), error.to_string()))?;
    if definition.kind != crate::types::TypeKind::Concrete {
        let concrete = CONSTRUCTORS
            .iter()
            .filter(|candidate| {
                system.definition(**candidate).is_ok_and(|found| {
                    found.kind == crate::types::TypeKind::Concrete
                        && found.parameters.len() == definition.parameters.len()
                })
            })
            .filter(|candidate| {
                let arguments = vec![TypeRef::DATA; definition.parameters.len()];
                candidate
                    .apply(arguments.clone())
                    .is(&constructor.apply(arguments))
            })
            .map(|candidate| format!("`{candidate}`"))
            .collect::<Vec<_>>();
        return Err(Diagnostic::typing(
            span,
            if concrete.is_empty() {
                format!("{constructor} is abstract; construct a concrete subtype")
            } else {
                format!(
                    "{constructor} is abstract; construct a concrete subtype: {}",
                    concrete.join(" or ")
                )
            },
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

/// `Graph(xs)` projects cards, hits or a ranking into a graph, and a graph
/// into itself.
fn check_graph(
    cx: &mut dyn CheckCx,
    args: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let [argument] = args else {
        return Err(Diagnostic::typing(
            span,
            "Graph expects one positional argument",
        ));
    };
    if argument.name.is_some() {
        return Err(Diagnostic::typing(
            span,
            "Graph expects one positional argument",
        ));
    }
    // A link that may not resolve projects to the graph of what it reaches.
    let inferred = cx.infer(&argument.value)?;
    let input = inferred.present();
    let card = |ty: &TypeRef| {
        ty.is(&TypeRef::DOC)
            || (ty.constructor.0 == "Hit"
                && ty.args.first().is_some_and(|arg| arg.is(&TypeRef::DOC)))
    };
    if input.is(&TypeRef::GRAPH) || card(input) || input.element().is_some_and(|ty| card(&ty)) {
        return Ok(TypeRef::GRAPH);
    }
    Err(Diagnostic::typing(
        argument.value.span.clone(),
        format!("a Graph is projected from cards, hits or a graph, not {input}"),
    ))
}

pub(crate) fn evaluate(
    cx: &mut dyn EvalCx,
    args: &[crate::execution::Arg],
    expected: &TypeRef,
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let constructor = expected.constructor;
    let mut inputs = Vec::new();
    for argument in args {
        inputs.push(cx.evaluate(&argument.value)?);
    }
    if constructor == GRAPH {
        let input = inputs
            .pop()
            .ok_or_else(|| Diagnostic::runtime(span.clone(), "Graph needs an argument"))?;
        return crate::extensions::graph::construct(cx.vault(), input, &span);
    }
    let element = |index: usize| expected.args.get(index).cloned().unwrap_or(TypeRef::NEVER);
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
        return Ok(Value::Map(Arc::new(map)));
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
