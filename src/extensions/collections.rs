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
use crate::reporting::Severity;
use crate::search::Ranking;
use crate::types::TypeRef;
use crate::types::{Key, Value};
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

const OPTION: crate::types::TypeConstructor = crate::types::TypeConstructor("Option");

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
            output: O::Reorder("T"),
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
            output: O::Image("R"),
        },
        summary: "Apply a function to every element; the result keeps the input's collection type.",
        purity: Purity::Pure,
        check: None,
        eval: eval_map,
    },
    Step {
        name: "of_type",
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
                name: "type",
                kind: A::TypeName("U"),
                required: true,
                positional: true,
                named: true,
            }],
            output: O::Image("U"),
        },
        summary: "Keep the elements of a type, narrowing the element type: filter(x => x | is(U)).",
        purity: Purity::Pure,
        check: Some(check_of_type),
        eval: eval_of_type,
    },
    Step {
        name: "is",
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
            input: P::Var("T"),
            arguments: &[ArgumentSpec {
                name: "type",
                kind: A::TypeName("U"),
                required: true,
                positional: true,
                named: true,
            }],
            output: O::Type(P::Type(crate::types::TypeConstructor("Bool"))),
        },
        summary: "Whether a value's runtime type is a type or one of its subtypes.",
        purity: Purity::Pure,
        check: Some(check_of_type),
        eval: eval_is,
    },
    Step {
        name: "somes",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Apply(OPTION, &[P::Var("T")])),
            arguments: &[],
            output: O::Image("T"),
        },
        summary: "Keep the present values of a collection of options, dropping the absent ones.",
        purity: Purity::Pure,
        check: None,
        eval: eval_somes,
    },
    Step {
        name: "group",
        spec: StepSpec {
            parameters: &[
                TypeParameter {
                    name: "T",
                    bound: None,
                },
                TypeParameter {
                    name: "K",
                    bound: None,
                },
            ],
            input: P::Collection(&P::Var("T")),
            arguments: &[ArgumentSpec {
                name: "by",
                kind: A::Lambda {
                    parameters: &[P::Var("T")],
                    result: P::Var("K"),
                },
                required: true,
                positional: true,
                named: true,
            }],
            output: O::Type(P::Apply(
                crate::types::collections::SET,
                &[P::Apply(
                    crate::types::builtin::GROUP,
                    &[P::Var("K"), P::Var("T")],
                )],
            )),
        },
        summary: "Partition occurrences by key into a set of groups, each a key and its items.",
        purity: Purity::Pure,
        check: None,
        eval: eval_group,
    },
    Step {
        name: "reduce",
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
            arguments: &[
                ArgumentSpec {
                    required: false,
                    ..ArgumentSpec::value("initial", P::Var("R"))
                },
                ArgumentSpec {
                    name: "by",
                    kind: A::Lambda {
                        parameters: &[P::Var("R"), P::Var("T")],
                        result: P::Var("R"),
                    },
                    required: true,
                    positional: true,
                    named: true,
                },
            ],
            output: O::Fold {
                initial: "initial",
                accumulator: "R",
                element: "T",
            },
        },
        summary: "Fold a sequence from the left, from an initial value or from its first element.",
        purity: Purity::Pure,
        check: Some(check_reduce),
        eval: eval_reduce,
    },
    Step {
        name: "sum",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[],
            output: O::Type(P::Var("T")),
        },
        summary: "Add every occurrence: reduce(0, +). An empty input sums to zero.",
        purity: Purity::Pure,
        check: Some(check_sum),
        eval: eval_sum,
    },
    Step {
        name: "min",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[],
            output: O::Type(P::Apply(OPTION, &[P::Var("T")])),
        },
        summary: "The least occurrence by value order: reduce(min). Absent over an empty input.",
        purity: Purity::Pure,
        check: Some(check_extreme),
        eval: eval_min,
    },
    Step {
        name: "max",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Collection(&P::Var("T")),
            arguments: &[],
            output: O::Type(P::Apply(OPTION, &[P::Var("T")])),
        },
        summary: "The greatest occurrence by value order: reduce(max). Absent over an empty input.",
        purity: Purity::Pure,
        check: Some(check_extreme),
        eval: eval_max,
    },
    Step {
        name: "debug",
        spec: StepSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Var("T"),
            arguments: &[
                ArgumentSpec {
                    required: false,
                    positional: false,
                    ..ArgumentSpec::value("label", P::Type(crate::types::TypeConstructor("String")))
                },
                ArgumentSpec {
                    required: false,
                    positional: false,
                    ..ArgumentSpec::value("level", P::Type(crate::types::TypeConstructor("String")))
                },
            ],
            output: O::Input,
        },
        // Pure on values: the program cannot read what it logged, so
        // removing every `debug` cannot change a result.
        summary: "Capture a value into the run's log and pass it on unchanged.",
        purity: Purity::Pure,
        check: Some(check_debug),
        eval: eval_debug,
    },
];

/// The severities `debug` may log at: what a value is, never that something
/// went wrong.
const DEBUG_LEVELS: [(&str, Severity); 2] = [("debug", Severity::Debug), ("info", Severity::Info)];

fn check_debug(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    _: Range<usize>,
) -> Result<(), Diagnostic> {
    let Some(level) = super::named(call.arguments, "level") else {
        return Ok(());
    };
    match &level.value.kind {
        crate::ast::Kind::Str(name) if DEBUG_LEVELS.iter().any(|(known, _)| known == name) => {
            Ok(())
        }
        _ => Err(Diagnostic::typing(
            level.value.span.clone(),
            "`level` is the literal \"debug\" or \"info\"",
        )),
    }
}

fn eval_debug(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let text = |cx: &mut dyn EvalCx, name: &str| -> Result<Option<String>, Diagnostic> {
        super::named(args, name)
            .map(|argument| match cx.evaluate(&argument.value)? {
                Value::Str(text) => Ok(text.to_string()),
                other => Err(Diagnostic::runtime(
                    argument.value.span.clone(),
                    format!("`{name}` is a String, not {}", other.type_of()),
                )),
            })
            .transpose()
    };
    let label = text(cx, "label")?;
    let severity = match text(cx, "level")? {
        None => Severity::Debug,
        Some(name) => DEBUG_LEVELS
            .iter()
            .find_map(|(known, severity)| (*known == name).then_some(*severity))
            .ok_or_else(|| Diagnostic::runtime(span.clone(), "checked level is unknown"))?,
    };
    let ty = input.type_of();
    let message = if ty.is_collection() {
        let count = elements(&input, "debug", &span)?.len();
        format!("{ty}, {count} element{}", if count == 1 { "" } else { "s" })
    } else {
        crate::reporting::shorten(&input.to_string())
    };
    let trace = cx.trace();
    cx.log(crate::reporting::Report::captured(
        crate::reporting::Capture {
            severity,
            span,
            label,
            message,
            ty: ty.to_string(),
            found: input.to_json(),
            shown: input.to_string(),
            trace,
        },
    ));
    Ok(input)
}

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
    let [argument] = args else {
        return Err(Diagnostic::runtime(span, "get needs a key"));
    };
    let key = cx.evaluate(&argument.value)?;
    match (input, key) {
        (Value::Map(map), key) => Ok(map.get(&key)),
        (Value::Data(data), Value::Str(key)) => crate::evaluation::data_key(&data, &key, &span),
        _ => Err(Diagnostic::runtime(span, "get needs a map")),
    }
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
    Ok(match narrowed_to(argument) {
        Some(wanted) => narrowed(&input, kept, wanted),
        None => rebuild(&input, kept),
    })
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
    // A set maps to its image, which collapses equal results.
    Ok(match input {
        Value::Set(..) => Value::set(mapped, argument.value.ty.clone()),
        _ => Value::list(mapped, argument.value.ty.clone()),
    })
}

fn eval_of_type(
    _: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "type").ok_or_else(|| wrong("of_type", "a type name", &span))?;
    let wanted = argument.value.ty.clone();
    // The kind was checked when the vault loaded; an authored entry alone
    // never put a card in this result.
    let kept: Vec<Value> = elements(&input, "of_type", &span)?
        .into_iter()
        .filter(|element| element.type_of().is(&wanted))
        .collect();
    Ok(narrowed(&input, kept, wanted))
}

fn eval_is(
    _: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "type").ok_or_else(|| wrong("is", "a type name", &span))?;
    Ok(Value::Bool(input.type_of().is(&argument.value.ty)))
}

/// The kept elements at the narrowed element type: a set stays a set, and
/// anything else is the sequence of its kept elements.
fn narrowed(input: &Value, kept: Vec<Value>, wanted: TypeRef) -> Value {
    match input {
        Value::Set(..) => Value::set(kept, wanted),
        _ => Value::list(kept, wanted),
    }
}

/// The type a `filter` predicate narrows its elements to: the `U` of a lambda
/// whose body is `x | is(U)` or `is(x, U)` on its own parameter.
pub(crate) fn narrowing(arguments: &[crate::ast::Arg]) -> Option<&str> {
    use crate::ast::Kind;
    let argument = named_or_first(arguments, "by")?;
    let Kind::Lambda { parameters, body } = &argument.value.kind else {
        return None;
    };
    let [parameter] = parameters.as_slice() else {
        return None;
    };
    let (subject, written) = match &body.kind {
        Kind::Pipe(subject, call) => match &call.kind {
            Kind::Call { callee, arguments } => (subject.as_ref(), (callee, arguments.as_slice())),
            _ => return None,
        },
        Kind::Call { callee, arguments } => match arguments.split_first() {
            Some((first, rest)) => (&first.value, (callee, rest)),
            None => return None,
        },
        _ => return None,
    };
    let (callee, [ty]) = written else {
        return None;
    };
    match (&subject.kind, &callee.kind, &ty.value.kind) {
        (Kind::Ident(name), Kind::Ident(is), Kind::Ident(ty))
            if name == parameter && is == "is" =>
        {
            Some(ty)
        }
        _ => None,
    }
}

/// The same detection over the checked tree, for evaluation.
fn narrowed_to(argument: &TypedArg) -> Option<TypeRef> {
    let TypedKind::Lambda { parameters, body } = &argument.value.kind else {
        return None;
    };
    let TypedKind::Step {
        step,
        input,
        arguments,
        ..
    } = &body.kind
    else {
        return None;
    };
    match (&input.kind, parameters.as_slice(), arguments.as_slice()) {
        (TypedKind::Ident(name), [parameter], [ty]) if step.name == "is" && name == parameter => {
            Some(ty.value.ty.clone())
        }
        _ => None,
    }
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
fn check_of_type(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    let element = &call.bindings["T"];
    let wanted = &call.bindings["U"];
    if !wanted.is(element) && !element.is(wanted) {
        return Err(Diagnostic::typing(
            span,
            format!("no {element} can be a {wanted}, so this is always false"),
        ));
    }
    Ok(())
}

fn eval_group(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "by").ok_or_else(|| wrong("group", "a key selector", &span))?;
    let item_type = element_type(&input);
    let key_type = argument.value.ty.clone();
    // Groups keep the order their keys were first seen in, and items their
    // input order; only the second is a promise, since the result is a set.
    let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
    for element in elements(&input, "group", &span)? {
        let key = cx.apply_lambda(argument, element.clone())?;
        match groups.iter_mut().find(|(seen, _)| *seen == key) {
            Some((_, items)) => items.push(element),
            None => groups.push((key, vec![element])),
        }
    }
    let group_type = crate::types::builtin::GROUP.apply([key_type.clone(), item_type.clone()]);
    Ok(Value::set(
        groups
            .into_iter()
            .map(|(key, items)| {
                Value::Group(Arc::new(crate::types::Group {
                    key,
                    items,
                    key_type: key_type.clone(),
                    item_type: item_type.clone(),
                }))
            })
            .collect(),
        group_type,
    ))
}

fn check_reduce(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if call.input.is_sequence() {
        return Ok(());
    }
    Err(Diagnostic::typing(
        span,
        format!(
            "`reduce` folds positions in order, and {} has none; sort it first",
            call.input
        ),
    ))
}

fn eval_reduce(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = super::named(arguments, "by")
        .or_else(|| {
            arguments
                .iter()
                .find(|argument| matches!(argument.value.kind, TypedKind::Lambda { .. }))
        })
        .ok_or_else(|| wrong("reduce", "a combining function", &span))?;
    let initial = super::named(arguments, "initial").or_else(|| {
        arguments.iter().find(|argument| {
            argument.name.is_none() && !matches!(argument.value.kind, TypedKind::Lambda { .. })
        })
    });
    let mut values = elements(&input, "reduce", &span)?.into_iter();
    let (mut accumulator, from_first) = match initial {
        Some(initial) => (cx.evaluate(&initial.value)?, false),
        None => match values.next() {
            Some(first) => (first, true),
            None => return Ok(Value::Absent(element_type(&input))),
        },
    };
    for value in values {
        accumulator = cx.apply_comparator(by, accumulator, value)?;
    }
    Ok(if from_first {
        Value::Present(Arc::new(accumulator))
    } else {
        accumulator
    })
}

fn check_sum(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    let element = &call.bindings["T"];
    if *element == TypeRef::INT || *element == TypeRef::FLOAT {
        return Ok(());
    }
    Err(Diagnostic::typing(
        span,
        format!("`sum` adds Int or Float occurrences, not {element}"),
    ))
}

fn eval_sum(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let floats = element_type(&input) == TypeRef::FLOAT;
    let mut whole: i64 = 0;
    let mut real: f64 = 0.0;
    // Added in input order, so a Float sum is the same on every run.
    for value in elements(&input, "sum", &span)? {
        match value {
            Value::Int(n) => {
                whole = whole
                    .checked_add(n)
                    .ok_or_else(|| Diagnostic::Overflow { span: span.clone() })?;
            }
            Value::Float(x) => real += x,
            _ => return Err(wrong("sum", "Int or Float occurrences", &span)),
        }
    }
    Ok(if floats {
        Value::Float(real)
    } else {
        Value::Int(whole)
    })
}

fn check_extreme(
    _: &dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    require_orderable(&call.bindings["T"], "min` / `max", &span)
}

fn eval_min(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    extreme(cx, input, arguments, span, Ordering::Less)
}

fn eval_max(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    extreme(cx, input, arguments, span, Ordering::Greater)
}

/// The first occurrence that no later one beats in the wanted direction.
fn extreme(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
    wanted: Ordering,
) -> Result<Value, Diagnostic> {
    let mut best: Option<Value> = None;
    for value in elements(&input, "min", &span)? {
        best = Some(match best {
            Some(held) if compare(&value.order_key(), &held.order_key()) != wanted => held,
            _ => value,
        });
    }
    Ok(match best {
        Some(value) => Value::Present(Arc::new(value)),
        None => Value::Absent(element_type(&input)),
    })
}

fn eval_somes(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let present = element_type(&input).present().clone();
    let kept = elements(&input, "somes", &span)?
        .into_iter()
        .filter_map(|value| match value {
            Value::Present(held) => Some(held.as_ref().clone()),
            _ => None,
        })
        .collect();
    Ok(narrowed(&input, kept, present))
}
