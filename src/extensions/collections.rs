//! The core's collection steps.
//!
//! These stay in the core rather than becoming an extension because they
//! operate on the core's own collection types and mention nothing above them.

use super::spec::{
    ArgumentKind as A, ArgumentSpec, Output as O, StepSpec, TypeParameter, TypePattern as P,
};
use super::{EvalCx, Purity, Step, named_or_first};
use crate::diagnostics::Diagnostic;
use crate::execution::{Arg as TypedArg, Kind as TypedKind};
use crate::search::Ranking;
use crate::types::TypeRef;
use crate::types::{Key, Value};
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

/// Every step the core provides, in the order help prints them.
pub(crate) static STEPS: &[Step] = &[
    Step {
        name: "size",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[],
            output: O::Type(P::Type(crate::types::TypeConstructor("Int"))),
        },
        summary: "Count all element occurrences.",
        purity: Purity::Pure,
        check: None,
        eval: eval_size,
    },
    Step {
        name: "contains",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("value", P::Var("T"))
            }],
            output: O::Type(P::Type(crate::types::TypeConstructor("Bool"))),
        },
        summary: "Whether a value occurs in the collection.",
        purity: Purity::Pure,
        check: None,
        eval: eval_contains,
    },
    Step {
        name: "get",
        spec: StepSpec {
            parameters: &[
                TypeParameter {
                    name: "K",
                    bound: None,
                },
                TypeParameter {
                    name: "V",
                    bound: None,
                },
            ],
            input: P::Apply(crate::types::collections::MAP, &[P::Var("K"), P::Var("V")]),
            arguments: &[ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("key", P::Var("K"))
            }],
            output: O::Type(P::Apply(
                crate::types::TypeConstructor("Option"),
                &[P::Var("V")],
            )),
        },
        summary: "Look up a key, yielding absence when it is missing.",
        purity: Purity::Pure,
        check: None,
        eval: eval_get,
    },
    Step {
        name: "count",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[ArgumentSpec {
                required: false,
                named: false,
                ..ArgumentSpec::value("value", P::Var("T"))
            }],
            output: O::Type(P::Type(crate::types::TypeConstructor("Int"))),
        },
        summary: "Count occurrences of a value; without an argument, count all occurrences.",
        purity: Purity::Pure,
        check: None,
        eval: eval_count,
    },
    Step {
        name: "sort",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[ArgumentSpec {
                name: "by",
                kind: A::Ordering(P::Var("T")),
                required: false,
                positional: true,
                named: true,
            }],
            output: O::Type(P::Apply(crate::types::collections::LIST, &[P::Var("T")])),
        },
        summary: "Order a collection by its elements' own key, or by one you supply.",
        purity: Purity::Pure,
        check: Some(check_sort),
        eval: eval_sort,
    },
    Step {
        name: "take",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[ArgumentSpec::value(
                "n",
                P::Type(crate::types::TypeConstructor("Int")),
            )],
            output: O::Prefix("T"),
        },
        summary: "The first n elements, in the elements' own order.",
        purity: Purity::Pure,
        check: Some(check_take),
        eval: eval_take,
    },
    Step {
        name: "filter",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[ArgumentSpec {
                name: "by",
                kind: A::Lambda {
                    parameters: &[P::Var("T")],
                    result: P::Type(crate::types::TypeConstructor("Bool")),
                },
                required: true,
                positional: true,
                named: true,
            }],
            output: O::Input,
        },
        summary: "Keep the elements a predicate accepts.",
        purity: Purity::Pure,
        check: None,
        eval: eval_filter,
    },
    Step {
        name: "map",
        spec: StepSpec {
            parameters: &[
                TypeParameter {
                    name: "T",
                    bound: None,
                },
                TypeParameter {
                    name: "R",
                    bound: None,
                },
            ],
            input: P::Collection(&P::Var("T")),
            arguments: &[ArgumentSpec {
                name: "by",
                kind: A::Lambda {
                    parameters: &[P::Var("T")],
                    result: P::Var("R"),
                },
                required: true,
                positional: true,
                named: true,
            }],
            output: O::Type(P::Apply(crate::types::collections::LIST, &[P::Var("R")])),
        },
        summary: "Apply a function to every element, yielding a sequence.",
        purity: Purity::Pure,
        check: None,
        eval: eval_map,
    },
    Step {
        name: "typed",
        spec: StepSpec {
            parameters: &[
                TypeParameter {
                    name: "T",
                    bound: None,
                },
                TypeParameter {
                    name: "U",
                    bound: None,
                },
            ],
            input: P::Collection(&P::Var("T")),
            arguments: &[ArgumentSpec {
                name: "as",
                kind: A::TypeName("U"),
                required: true,
                positional: true,
                named: true,
            }],
            output: O::WithElement("U"),
        },
        summary: "Keep the elements that are of a type, checked rather than trusted.",
        purity: Purity::Pure,
        check: Some(check_typed),
        eval: eval_typed,
    },
];

fn eval_count(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let elements = elements(&input, "count", &span)?;
    let count = match args.first() {
        Some(arg) => {
            let wanted = cx.evaluate(&arg.value)?;
            elements
                .iter()
                .filter(|element| **element == wanted)
                .count()
        }
        None => elements.len(),
    };
    i64::try_from(count)
        .map(Value::Int)
        .map_err(|_| Diagnostic::runtime(span, "collection size exceeds Int"))
}

fn eval_size(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    eval_count(cx, input, args, span)
}

fn eval_contains(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let result = eval_count(cx, input, args, span)?;
    Ok(Value::Bool(
        matches!(result, Value::Int(count) if count > 0),
    ))
}

fn eval_get(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let Value::Map(map) = input else {
        return Err(Diagnostic::runtime(span, "get needs a map"));
    };
    let [argument] = args else {
        return Err(Diagnostic::runtime(span, "get needs a key"));
    };
    Ok(map.get(&cx.evaluate(&argument.value)?))
}

fn eval_sort(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let mut elements = elements(&input, "sort", &span)?;
    match named_or_first(arguments, "by") {
        Some(argument) if matches!(&argument.value.kind, TypedKind::Lambda { parameters, .. } if parameters.len() == 2) =>
        {
            // A fallible comparison must never be hidden inside an infallible
            // sorting callback. Insertion sort also preserves equal-key positions.
            for index in 1..elements.len() {
                let mut position = index;
                while position > 0 {
                    let order = cx.apply_comparator(
                        argument,
                        elements[position - 1].clone(),
                        elements[position].clone(),
                    )?;
                    let Value::Ordering(order) = order else {
                        return Err(Diagnostic::runtime(
                            span,
                            "a comparison must return Ordering",
                        ));
                    };
                    if order != Ordering::Greater {
                        break;
                    }
                    elements.swap(position - 1, position);
                    position -= 1;
                }
            }
        }
        Some(argument) => {
            let mut keyed = Vec::new();
            for element in elements {
                let key = cx.apply_lambda(argument, element.clone())?;
                keyed.push((key.order_key(), element));
            }
            keyed.sort_by(|left, right| compare(&left.0, &right.0));
            elements = keyed.into_iter().map(|(_, element)| element).collect();
        }
        None => elements.sort_by(|left, right| compare(&left.order_key(), &right.order_key())),
    }
    Ok(Value::list(elements, element_type(&input)))
}

fn eval_take(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "n").ok_or_else(|| wrong("take", "a count: `take(5)`", &span))?;
    let Value::Int(count) = cx.evaluate(&argument.value)? else {
        return Err(wrong("take", "an Int count", &span));
    };
    let count = usize::try_from(count).map_err(|_| {
        Diagnostic::runtime(argument.value.span.clone(), "a count cannot be negative")
    })?;
    if let Value::Ranking(ranking) = &input {
        // Already ordered by score, and that order is the point.
        return Ok(Value::Ranking(Arc::new(Ranking {
            hits: ranking.hits.iter().take(count).cloned().collect(),
            retrieval: Arc::clone(&ranking.retrieval),
        })));
    }
    let mut elements = elements(&input, "take", &span)?;
    // A set has no order, so the prefix comes from the elements' own. A
    // sequence already has one, and imposing another here would discard the
    // `sort` that produced it.
    if matches!(input, Value::Set(..)) {
        elements.sort_by(|left, right| compare(&left.order_key(), &right.order_key()));
    }
    elements.truncate(count);
    Ok(Value::list(elements, element_type(&input)))
}

fn eval_filter(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "by").ok_or_else(|| wrong("filter", "a predicate", &span))?;
    let mut kept = Vec::new();
    for element in elements(&input, "filter", &span)? {
        let verdict = cx.apply_lambda(argument, element.clone())?;
        if verdict.truth().ok_or_else(|| {
            Diagnostic::typing(
                argument.value.span.clone(),
                "a filter predicate returns Bool",
            )
        })? {
            kept.push(element);
        }
    }
    Ok(rebuild(&input, kept))
}

fn eval_map(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "by").ok_or_else(|| wrong("map", "a function", &span))?;
    let mut mapped = Vec::new();
    for element in elements(&input, "map", &span)? {
        mapped.push(cx.apply_lambda(argument, element)?);
    }
    Ok(Value::list(mapped, argument.value.ty.clone()))
}

fn eval_typed(
    _: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "as").ok_or_else(|| wrong("typed", "a type name", &span))?;
    let wanted = argument.value.ty.clone();
    // The kind was checked when the vault loaded; an authored entry alone
    // never put a card in this result.
    let kept: Vec<Value> = elements(&input, "typed", &span)?
        .into_iter()
        .filter(|element| element.type_of().is(&wanted))
        .collect();
    Ok(match input {
        Value::Set(..) => Value::set(kept, wanted),
        _ => rebuild(&input, kept).viewed_as(&input.type_of().with_element(wanted)),
    })
}

fn require_orderable(element: &TypeRef, name: &str, span: &Range<usize>) -> Result<(), Diagnostic> {
    if element.is_orderable() {
        return Ok(());
    }
    Err(Diagnostic::typing(
        span.clone(),
        format!(
            "`{name}` needs elements with an order of their own, and {element} has none; \
             sort with an explicit key first"
        ),
    ))
}

/// The elements of a value, or the failure that says it holds none.
pub(crate) fn elements(
    input: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<Vec<Value>, Diagnostic> {
    input
        .elements()
        .ok_or_else(|| wrong(name, "a collection", span))
}

/// The shape every step's "wrong input" failure is written in.
pub(crate) fn wrong(name: &str, what: &str, span: &Range<usize>) -> Diagnostic {
    Diagnostic::typing(span.clone(), format!("`{name}` needs {what}"))
}

/// The element type a collection value carries.
pub(crate) fn element_type(input: &Value) -> TypeRef {
    input.type_of().element().unwrap_or(TypeRef::DATA)
}

/// Keep a filtered collection in the shape it came in.
fn rebuild(input: &Value, kept: Vec<Value>) -> Value {
    match input {
        Value::Set(_, element) => Value::set(kept, element.clone()),
        Value::Ranking(ranking) => Value::Ranking(Arc::new(Ranking {
            hits: kept
                .into_iter()
                .filter_map(|value| match value {
                    Value::Hit(hit) => Some(hit.as_ref().clone()),
                    _ => None,
                })
                .collect(),
            retrieval: Arc::clone(&ranking.retrieval),
        })),
        _ => Value::list(kept, element_type(input)),
    }
}

fn compare(left: &Option<Key>, right: &Option<Key>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.compare(right),
        _ => Ordering::Equal,
    }
}

fn check_sort(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if call.arguments.is_empty() {
        require_orderable(&call.bindings["T"], "sort", &span)?;
    }
    Ok(())
}
fn check_take(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if !call.input.is_sequence() {
        require_orderable(&call.bindings["T"], "take", &span)?;
    }
    Ok(())
}
fn check_typed(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    let element = &call.bindings["T"];
    let wanted = &call.bindings["U"];
    if !wanted.is(element) && !element.is(wanted) {
        return Err(Diagnostic::typing(
            span,
            format!("no {element} can be a {wanted}, so this selects nothing"),
        ));
    }
    Ok(())
}
