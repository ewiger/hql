//! The `seq` module: the sequence, set and map operations beyond the core's
//! membership, order and lookup. Pairs, `Pair<A, B>` in `std/core.hql`, are
//! how `zip`, `enumerate`, `entries` and `join` hold two values together.

use super::spec::{
    ArgumentKind as A, ArgumentSpec, OperationSpec, Output as O, TypeParameter, TypePattern as P,
};
use super::{EvalCx, Operation, Purity, named_or_first};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::collections::{LIST, MAP, SEQ, SET};
use crate::types::{MapKind, MapValue, Record, TypeConstructor, TypeRef, Value};
use std::ops::Range;
use std::sync::Arc;

use super::collections::{elements, pending, produced, wrong};

const INT: TypeConstructor = TypeConstructor("Int");
const BOOL: TypeConstructor = TypeConstructor("Bool");
const OPTION: TypeConstructor = TypeConstructor("Option");
/// The constructor of a pair, `type Pair<A, B> { first : A, second : B }`.
pub const PAIR: TypeConstructor = TypeConstructor("Pair");

const T: TypeParameter = TypeParameter {
    name: "T",
    bound: None,
};
const U: TypeParameter = TypeParameter {
    name: "U",
    bound: None,
};
const K: TypeParameter = TypeParameter {
    name: "K",
    bound: None,
};
const V: TypeParameter = TypeParameter {
    name: "V",
    bound: None,
};
const SEQ_T: P = P::Apply(SEQ, &[P::Var("T")]);
const PREDICATE: ArgumentSpec = ArgumentSpec {
    name: "by",
    kind: A::Lambda {
        parameters: &[P::Var("T")],
        result: P::Type(BOOL),
    },
    required: true,
    positional: true,
    named: true,
    variadic: false,
};

const fn op(
    name: &'static str,
    summary: &'static str,
    parameters: &'static [TypeParameter],
    input: P,
    arguments: &'static [ArgumentSpec],
    output: O,
    eval: super::Eval,
) -> Operation {
    Operation {
        name,
        spec: OperationSpec {
            parameters,
            input,
            arguments,
            output,
        },
        summary,
        purity: Purity::Pure,
        check: None,
        eval,
    }
}

/// Every step the `seq` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    op(
        "first",
        "The element at position 0, or absence over an empty sequence.",
        &[T],
        SEQ_T,
        &[],
        O::Type(P::Apply(OPTION, &[P::Var("T")])),
        eval_first,
    ),
    op(
        "last",
        "The element at the last position, or absence over an empty sequence.",
        &[T],
        SEQ_T,
        &[],
        O::Type(P::Apply(OPTION, &[P::Var("T")])),
        eval_last,
    ),
    op(
        "reverse",
        "The elements in the opposite order.",
        &[T],
        SEQ_T,
        &[],
        O::Reorder("T"),
        eval_reverse,
    ),
    op(
        "distinct",
        "Each value once, at its first position.",
        &[T],
        SEQ_T,
        &[],
        O::Prefix("T"),
        eval_distinct,
    ),
    op(
        "zip",
        "Pairs of elements at equal positions, as long as the shorter sequence.",
        &[T, U],
        SEQ_T,
        &[ArgumentSpec {
            named: false,
            ..ArgumentSpec::value("other", P::Apply(SEQ, &[P::Var("U")]))
        }],
        O::Type(P::Apply(
            LIST,
            &[P::Apply(PAIR, &[P::Var("T"), P::Var("U")])],
        )),
        eval_zip,
    ),
    op(
        "enumerate",
        "Each element paired with its position, counting from 0.",
        &[T],
        SEQ_T,
        &[],
        O::Type(P::Apply(
            LIST,
            &[P::Apply(PAIR, &[P::Type(INT), P::Var("T")])],
        )),
        eval_enumerate,
    ),
    op(
        "any",
        "Whether a predicate holds of some occurrence; false over an empty input.",
        &[T],
        P::Collection(&P::Var("T")),
        &[PREDICATE],
        O::Type(P::Type(BOOL)),
        eval_any,
    ),
    op(
        "all",
        "Whether a predicate holds of every occurrence; true over an empty input.",
        &[T],
        P::Collection(&P::Var("T")),
        &[PREDICATE],
        O::Type(P::Type(BOOL)),
        eval_all,
    ),
    op(
        "find",
        "The first element a predicate accepts, or absence.",
        &[T],
        SEQ_T,
        &[PREDICATE],
        O::Type(P::Apply(OPTION, &[P::Var("T")])),
        eval_find,
    ),
    op(
        "position",
        "The position of the first element a predicate accepts, or absence.",
        &[T],
        SEQ_T,
        &[PREDICATE],
        O::Type(P::Apply(OPTION, &[P::Type(INT)])),
        eval_position,
    ),
    op(
        "take_while",
        "The leading values a predicate accepts, stopping at the first it rejects.",
        &[T],
        P::Iterable(&P::Var("T")),
        &[PREDICATE],
        O::Prefix("T"),
        eval_take_while,
    ),
    op(
        "drop_while",
        "The values after the leading ones a predicate accepts.",
        &[T],
        SEQ_T,
        &[PREDICATE],
        O::Prefix("T"),
        eval_drop_while,
    ),
    op(
        "window",
        "Every run of n consecutive elements, in order.",
        &[T],
        SEQ_T,
        &[ArgumentSpec {
            named: false,
            ..ArgumentSpec::value("n", P::Type(INT))
        }],
        O::Type(P::Apply(LIST, &[P::Apply(LIST, &[P::Var("T")])])),
        eval_window,
    ),
    op(
        "partition",
        "The elements a predicate accepts, and then the ones it rejects, each in order.",
        &[T],
        SEQ_T,
        &[PREDICATE],
        O::Type(P::Apply(
            PAIR,
            &[
                P::Apply(LIST, &[P::Var("T")]),
                P::Apply(LIST, &[P::Var("T")]),
            ],
        )),
        eval_partition,
    ),
    op(
        "is_empty",
        "Whether the collection has no occurrences.",
        &[T],
        P::Collection(&P::Var("T")),
        &[],
        O::Type(P::Type(BOOL)),
        eval_is_empty,
    ),
    op(
        "range",
        "The Ints from the start up to, not including, an end, by a step of 1 or as given.",
        &[],
        P::Type(INT),
        &[
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("end", P::Type(INT))
            },
            ArgumentSpec {
                required: false,
                ..ArgumentSpec::value("step", P::Type(INT))
            },
        ],
        O::Type(P::Apply(LIST, &[P::Type(INT)])),
        eval_range,
    ),
    op(
        "join",
        "Every pair of an element and one of another collection that a predicate relates.",
        &[T, U],
        P::Collection(&P::Var("T")),
        &[
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("other", P::Collection(&P::Var("U")))
            },
            ArgumentSpec {
                name: "by",
                kind: A::Lambda {
                    parameters: &[P::Var("T"), P::Var("U")],
                    result: P::Type(BOOL),
                },
                required: true,
                positional: true,
                named: true,
                variadic: false,
            },
        ],
        O::Type(P::Apply(
            LIST,
            &[P::Apply(PAIR, &[P::Var("T"), P::Var("U")])],
        )),
        eval_join,
    ),
    op(
        "min_by",
        "The element whose key is least, or absence over an empty input.",
        &[T],
        P::Collection(&P::Var("T")),
        &[ArgumentSpec {
            name: "by",
            kind: A::Ordering(P::Var("T")),
            required: true,
            positional: true,
            named: true,
            variadic: false,
        }],
        O::Type(P::Apply(OPTION, &[P::Var("T")])),
        eval_min_by,
    ),
    op(
        "max_by",
        "The element whose key is greatest, or absence over an empty input.",
        &[T],
        P::Collection(&P::Var("T")),
        &[ArgumentSpec {
            name: "by",
            kind: A::Ordering(P::Var("T")),
            required: true,
            positional: true,
            named: true,
            variadic: false,
        }],
        O::Type(P::Apply(OPTION, &[P::Var("T")])),
        eval_max_by,
    ),
    op(
        "to_map",
        "A map from each element's key to its value; two elements with one key are refused.",
        &[T, K, V],
        P::Collection(&P::Var("T")),
        &[
            ArgumentSpec {
                name: "key",
                kind: A::Lambda {
                    parameters: &[P::Var("T")],
                    result: P::Var("K"),
                },
                required: true,
                positional: true,
                named: true,
                variadic: false,
            },
            ArgumentSpec {
                name: "value",
                kind: A::Lambda {
                    parameters: &[P::Var("T")],
                    result: P::Var("V"),
                },
                required: true,
                positional: true,
                named: true,
                variadic: false,
            },
        ],
        O::Type(P::Apply(MAP, &[P::Var("K"), P::Var("V")])),
        eval_to_map,
    ),
    op(
        "with",
        "The map with one key set to a value, added or replaced.",
        &[K, V],
        P::Apply(MAP, &[P::Var("K"), P::Var("V")]),
        &[
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("key", P::Var("K"))
            },
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("value", P::Var("V"))
            },
        ],
        O::Input,
        eval_with,
    ),
    op(
        "without",
        "The map without one key, whether or not it held it.",
        &[K, V],
        P::Apply(MAP, &[P::Var("K"), P::Var("V")]),
        &[ArgumentSpec {
            named: false,
            ..ArgumentSpec::value("key", P::Var("K"))
        }],
        O::Input,
        eval_without,
    ),
    op(
        "entries",
        "Each association as a pair of its key and its value.",
        &[K, V],
        P::Apply(MAP, &[P::Var("K"), P::Var("V")]),
        &[],
        O::Type(P::Apply(
            LIST,
            &[P::Apply(PAIR, &[P::Var("K"), P::Var("V")])],
        )),
        eval_entries,
    ),
    op(
        "values",
        "The values of a map, in its key order.",
        &[K, V],
        P::Apply(MAP, &[P::Var("K"), P::Var("V")]),
        &[],
        O::Type(P::Apply(LIST, &[P::Var("V")])),
        eval_values,
    ),
    op(
        "intersection",
        "The members of both sets.",
        &[T],
        P::Apply(SET, &[P::Var("T")]),
        &[ArgumentSpec {
            named: false,
            ..ArgumentSpec::value("other", P::Apply(SET, &[P::Var("T")]))
        }],
        O::Input,
        eval_intersection,
    ),
    op(
        "difference",
        "The members of the set that the other does not hold.",
        &[T],
        P::Apply(SET, &[P::Var("T")]),
        &[ArgumentSpec {
            named: false,
            ..ArgumentSpec::value("other", P::Apply(SET, &[P::Var("T")]))
        }],
        O::Input,
        eval_difference,
    ),
];

/// A `Pair<A, B>` value.
pub(crate) fn pair(first: Value, second: Value, a: TypeRef, b: TypeRef) -> Value {
    Value::Record(Arc::new(Record {
        ty: PAIR.apply([a, b]),
        fields: vec![("first".to_owned(), first), ("second".to_owned(), second)],
    }))
}

fn element_of(input: &Value) -> TypeRef {
    super::collections::element_type(input)
}

fn optional(value: Option<Value>, element: TypeRef) -> Value {
    match value {
        Some(value) => Value::Present(Arc::new(value)),
        None => Value::Absent(element),
    }
}

fn holds(
    cx: &mut dyn EvalCx,
    by: &TypedArg,
    element: Value,
    name: &str,
) -> Result<bool, Diagnostic> {
    cx.apply_lambda(by, element)?.truth().ok_or_else(|| {
        Diagnostic::typing(
            by.value.span.clone(),
            format!("a `{name}` predicate returns Bool"),
        )
    })
}

fn predicate<'a>(
    arguments: &'a [TypedArg],
    name: &str,
    span: &Range<usize>,
) -> Result<&'a TypedArg, Diagnostic> {
    named_or_first(arguments, "by").ok_or_else(|| wrong(name, "a predicate", span))
}

fn eval_first(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let element = element_of(&input);
    let first = elements(&input, "first", &span)?.into_iter().next();
    Ok(optional(first, element))
}

fn eval_last(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let element = element_of(&input);
    let last = elements(&input, "last", &span)?.pop();
    Ok(optional(last, element))
}

fn eval_reverse(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let element = element_of(&input);
    let mut values = elements(&input, "reverse", &span)?;
    values.reverse();
    Ok(Value::list(values, element))
}

fn eval_distinct(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let element = element_of(&input);
    let mut kept: Vec<Value> = Vec::new();
    for value in elements(&input, "distinct", &span)? {
        if !kept.contains(&value) {
            kept.push(value);
        }
    }
    Ok(Value::list(kept, element))
}

fn eval_zip(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let other = arguments
        .first()
        .ok_or_else(|| wrong("zip", "another sequence", &span))?;
    let other_value = cx.evaluate(&other.value)?;
    let (a, b) = (element_of(&input), element_of(&other_value));
    let pairs = elements(&input, "zip", &span)?
        .into_iter()
        .zip(elements(&other_value, "zip", &span)?)
        .map(|(first, second)| pair(first, second, a.clone(), b.clone()))
        .collect();
    Ok(Value::list(pairs, PAIR.apply([a, b])))
}

fn eval_enumerate(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let element = element_of(&input);
    let pairs = elements(&input, "enumerate", &span)?
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            pair(
                Value::Int(i64::try_from(index).unwrap_or(i64::MAX)),
                value,
                TypeRef::INT,
                element.clone(),
            )
        })
        .collect();
    Ok(Value::list(pairs, PAIR.apply([TypeRef::INT, element])))
}

fn eval_any(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = predicate(arguments, "any", &span)?;
    for element in elements(&input, "any", &span)? {
        if holds(cx, by, element, "any")? {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn eval_all(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = predicate(arguments, "all", &span)?;
    for element in elements(&input, "all", &span)? {
        if !holds(cx, by, element, "all")? {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn eval_find(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = predicate(arguments, "find", &span)?;
    let element = element_of(&input);
    for value in elements(&input, "find", &span)? {
        if holds(cx, by, value.clone(), "find")? {
            return Ok(Value::Present(Arc::new(value)));
        }
    }
    Ok(Value::Absent(element))
}

fn eval_position(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = predicate(arguments, "position", &span)?;
    for (index, value) in elements(&input, "position", &span)?.into_iter().enumerate() {
        if holds(cx, by, value, "position")? {
            return Ok(Value::Present(Arc::new(Value::Int(
                i64::try_from(index).unwrap_or(i64::MAX),
            ))));
        }
    }
    Ok(Value::Absent(TypeRef::INT))
}

fn eval_take_while(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = predicate(arguments, "take_while", &span)?;
    let mut kept = Vec::new();
    // A stream is asked for values until the predicate fails, so a stream
    // that never ends still yields a finite prefix.
    if let Some(traversal) = pending(&input) {
        let element = traversal.element.clone();
        let traversal = Arc::clone(traversal);
        cx.traverse(&traversal, &mut |cx, value| {
            if holds(cx, by, value.clone(), "take_while")? {
                kept.push(value);
                Ok(true)
            } else {
                Ok(false)
            }
        })?;
        return Ok(Value::list(kept, element));
    }
    let element = element_of(&input);
    for value in elements(&input, "take_while", &span)? {
        if !holds(cx, by, value.clone(), "take_while")? {
            break;
        }
        kept.push(value);
    }
    Ok(Value::list(kept, element))
}

fn eval_drop_while(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = predicate(arguments, "drop_while", &span)?;
    let element = element_of(&input);
    let mut kept = Vec::new();
    let mut dropping = true;
    for value in elements(&input, "drop_while", &span)? {
        if dropping && holds(cx, by, value.clone(), "drop_while")? {
            continue;
        }
        dropping = false;
        kept.push(value);
    }
    Ok(Value::list(kept, element))
}

fn eval_window(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let n = arguments
        .first()
        .ok_or_else(|| wrong("window", "a width", &span))?;
    let Value::Int(n) = cx.evaluate(&n.value)? else {
        return Err(wrong("window", "an Int width", &span));
    };
    let n = usize::try_from(n)
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "a window is at least 1 wide"))?;
    let element = element_of(&input);
    let values = elements(&input, "window", &span)?;
    let windows = values
        .windows(n)
        .map(|window| Value::list(window.to_vec(), element.clone()))
        .collect();
    Ok(Value::list(windows, TypeRef::list(element)))
}

fn eval_partition(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = predicate(arguments, "partition", &span)?;
    let element = element_of(&input);
    let (mut accepted, mut rejected) = (Vec::new(), Vec::new());
    for value in elements(&input, "partition", &span)? {
        if holds(cx, by, value.clone(), "partition")? {
            accepted.push(value);
        } else {
            rejected.push(value);
        }
    }
    let list = TypeRef::list(element.clone());
    Ok(pair(
        Value::list(accepted, element.clone()),
        Value::list(rejected, element),
        list.clone(),
        list,
    ))
}

fn eval_is_empty(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Bool(elements(&input, "is_empty", &span)?.is_empty()))
}

fn eval_range(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let Value::Int(start) = input else {
        return Err(wrong("range", "an Int start", &span));
    };
    let end = named_or_first(arguments, "end").ok_or_else(|| wrong("range", "an end", &span))?;
    let Value::Int(end) = cx.evaluate(&end.value)? else {
        return Err(wrong("range", "an Int end", &span));
    };
    let step = match super::named(arguments, "step").or_else(|| arguments.get(1)) {
        Some(step) => match cx.evaluate(&step.value)? {
            Value::Int(0) => {
                return Err(Diagnostic::runtime(span, "a range's step is not 0"));
            }
            Value::Int(step) => step,
            _ => return Err(wrong("range", "an Int step", &span)),
        },
        None => 1,
    };
    let mut values = Vec::new();
    let mut at = start;
    while (step > 0 && at < end) || (step < 0 && at > end) {
        values.push(Value::Int(at));
        let Some(next) = at.checked_add(step) else {
            break;
        };
        at = next;
    }
    Ok(Value::list(values, TypeRef::INT))
}

fn eval_join(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let other = arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some("other"))
        .or_else(|| {
            arguments
                .iter()
                .find(|argument| argument.name.is_none() && !is_function(argument))
        })
        .ok_or_else(|| wrong("join", "another collection", &span))?;
    let by = super::named(arguments, "by")
        .or_else(|| arguments.iter().find(|argument| is_function(argument)))
        .ok_or_else(|| wrong("join", "a predicate over a pair", &span))?;
    let other_value = cx.evaluate(&other.value)?;
    let (a, b) = (element_of(&input), element_of(&other_value));
    let right = elements(&other_value, "join", &span)?;
    let mut pairs = Vec::new();
    for left in elements(&input, "join", &span)? {
        for candidate in &right {
            let related = cx.apply_comparator(by, left.clone(), candidate.clone())?;
            if related.truth().ok_or_else(|| {
                Diagnostic::typing(by.value.span.clone(), "a `join` predicate returns Bool")
            })? {
                pairs.push(pair(left.clone(), candidate.clone(), a.clone(), b.clone()));
            }
        }
    }
    Ok(Value::list(pairs, PAIR.apply([a, b])))
}

fn is_function(argument: &TypedArg) -> bool {
    matches!(argument.value.kind, crate::execution::Kind::Lambda { .. })
        || argument.value.ty.as_function().is_some()
}

fn extreme_by(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
    name: &str,
    greatest: bool,
) -> Result<Value, Diagnostic> {
    let by = named_or_first(arguments, "by").ok_or_else(|| wrong(name, "a key", &span))?;
    let element = element_of(&input);
    let comparator = super::collections::arity_of(by) == Some(2);
    let mut best: Option<(Value, Option<Value>)> = None;
    for value in elements(&input, name, &span)? {
        let key = if comparator {
            None
        } else {
            Some(cx.apply_lambda(by, value.clone())?)
        };
        let better = match &best {
            None => true,
            Some((held, held_key)) => {
                let order = if comparator {
                    match cx.apply_comparator(by, value.clone(), held.clone())? {
                        Value::Ordering(order) => order,
                        _ => std::cmp::Ordering::Equal,
                    }
                } else {
                    match (&key, held_key) {
                        (Some(key), Some(held_key)) => cx.compare_values(key, held_key, &span)?,
                        _ => std::cmp::Ordering::Equal,
                    }
                };
                if greatest {
                    order.is_gt()
                } else {
                    order.is_lt()
                }
            }
        };
        if better {
            best = Some((value, key));
        }
    }
    Ok(optional(best.map(|(value, _)| value), element))
}

fn eval_min_by(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    extreme_by(cx, input, arguments, span, "min_by", false)
}

fn eval_max_by(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    extreme_by(cx, input, arguments, span, "max_by", true)
}

fn map_value(
    kind: MapKind,
    keys: Vec<Value>,
    values: Vec<Value>,
    key: TypeRef,
    value: TypeRef,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    MapValue::new(kind, keys, values, key, value)
        .map(|map| Value::Map(Arc::new(map)))
        .map_err(|error| Diagnostic::runtime(span.clone(), error.to_string()))
}

fn eval_to_map(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let (key, value) = match (
        super::named(arguments, "key"),
        super::named(arguments, "value"),
    ) {
        (Some(key), Some(value)) => (key, value),
        _ => {
            let positional: Vec<&TypedArg> = arguments
                .iter()
                .filter(|argument| argument.name.is_none())
                .collect();
            match positional.as_slice() {
                [key, value] => (*key, *value),
                _ => return Err(wrong("to_map", "a key and a value function", &span)),
            }
        }
    };
    let (mut keys, mut values) = (Vec::new(), Vec::new());
    for element in elements(&input, "to_map", &span)? {
        keys.push(cx.apply_lambda(key, element.clone())?);
        values.push(cx.apply_lambda(value, element)?);
    }
    map_value(
        MapKind::Unordered,
        keys,
        values,
        produced(key),
        produced(value),
        &span,
    )
}

fn map_parts(input: &Value, name: &str, span: &Range<usize>) -> Result<Arc<MapValue>, Diagnostic> {
    match input {
        Value::Map(map) => Ok(Arc::clone(map)),
        _ => Err(wrong(name, "a map", span)),
    }
}

fn eval_with(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let map = map_parts(&input, "with", &span)?;
    let [key, value] = arguments else {
        return Err(wrong("with", "a key and a value", &span));
    };
    let key = cx.evaluate(&key.value)?;
    let value = cx.evaluate(&value.value)?;
    let mut entries: Vec<(Value, Value)> = map.entries().to_vec();
    match entries.iter_mut().find(|(held, _)| *held == key) {
        Some(entry) => entry.1 = value,
        None => entries.push((key, value)),
    }
    let ty = map.type_of();
    let kind = MapKind::of(ty.constructor).unwrap_or(MapKind::Unordered);
    let (keys, values) = entries.into_iter().unzip();
    map_value(
        kind,
        keys,
        values,
        ty.args.first().cloned().unwrap_or(TypeRef::DATA),
        ty.args.get(1).cloned().unwrap_or(TypeRef::DATA),
        &span,
    )
}

fn eval_without(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let map = map_parts(&input, "without", &span)?;
    let [key] = arguments else {
        return Err(wrong("without", "a key", &span));
    };
    let key = cx.evaluate(&key.value)?;
    let ty = map.type_of();
    let kind = MapKind::of(ty.constructor).unwrap_or(MapKind::Unordered);
    let (keys, values) = map
        .entries()
        .iter()
        .filter(|(held, _)| *held != key)
        .cloned()
        .unzip();
    map_value(
        kind,
        keys,
        values,
        ty.args.first().cloned().unwrap_or(TypeRef::DATA),
        ty.args.get(1).cloned().unwrap_or(TypeRef::DATA),
        &span,
    )
}

fn eval_entries(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let map = map_parts(&input, "entries", &span)?;
    let ty = map.type_of();
    let (k, v) = (
        ty.args.first().cloned().unwrap_or(TypeRef::DATA),
        ty.args.get(1).cloned().unwrap_or(TypeRef::DATA),
    );
    let pairs = map
        .entries()
        .iter()
        .map(|(key, value)| pair(key.clone(), value.clone(), k.clone(), v.clone()))
        .collect();
    Ok(Value::list(pairs, PAIR.apply([k, v])))
}

fn eval_values(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let map = map_parts(&input, "values", &span)?;
    let v = map.type_of().args.get(1).cloned().unwrap_or(TypeRef::DATA);
    Ok(Value::list(
        map.entries()
            .iter()
            .map(|(_, value)| value.clone())
            .collect(),
        v,
    ))
}

fn set_algebra(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
    name: &str,
    keep_shared: bool,
) -> Result<Value, Diagnostic> {
    let other = arguments
        .first()
        .ok_or_else(|| wrong(name, "another set", &span))?;
    let other = cx.evaluate(&other.value)?;
    let right = elements(&other, name, &span)?;
    let element = element_of(&input);
    let kept = elements(&input, name, &span)?
        .into_iter()
        .filter(|value| right.contains(value) == keep_shared)
        .collect();
    Ok(Value::set(kept, element))
}

fn eval_intersection(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    set_algebra(cx, input, arguments, span, "intersection", true)
}

fn eval_difference(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    set_algebra(cx, input, arguments, span, "difference", false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_has_a_coherent_spec() {
        for step in STEPS {
            step.spec
                .validate()
                .unwrap_or_else(|error| panic!("{}: {error}", step.name));
        }
    }
}
