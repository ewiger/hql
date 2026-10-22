//! The core's collection steps.
//!
//! These stay in the core rather than becoming an extension because they
//! operate on the core's own collection types and mention nothing above them.

use super::Signature;
use super::{EvalCx, Operation, Purity, named_or_first};
use crate::diagnostics::Diagnostic;
use crate::execution::{Arg as TypedArg, Kind as TypedKind};
use crate::search::Ranking;
use crate::types::TypeRef;
use crate::types::{Key, Value};
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

use crate::traversal::{Source, Traversal};

/// The steps that consume a traversal one value at a time and so keep it
/// lazy. Every other step receives a traversal's values, forced once.
pub(crate) const LAZY: [&str; 18] = [
    "map",
    "filter",
    "take",
    "take_while",
    "drop",
    "collect",
    "emit",
    "flat_map",
    "format",
    "text",
    "markdown",
    "hmd",
    "json",
    "yaml",
    "csv",
    "d2",
    "mermaid",
    "sink",
];

/// A traversal that has not been forced, which a lazy step wraps.
pub(crate) fn pending(input: &Value) -> Option<&Arc<Traversal>> {
    match input {
        Value::Traversal(traversal) if traversal.forced().is_none() => Some(traversal),
        _ => None,
    }
}

fn wrapped(input: &Arc<Traversal>, element: TypeRef, source: Source) -> Value {
    Value::Traversal(Arc::new(Traversal::new(element, input.finite, source)))
}

/// Every step the core provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "size",
        spec: Signature::Declared,
        summary: "Count all element occurrences.",
        purity: Purity::Pure,
        check: None,
        eval: eval_size,
    },
    Operation {
        name: "contains",
        spec: Signature::Declared,
        summary: "Whether a value occurs in the collection.",
        purity: Purity::Pure,
        check: None,
        eval: eval_contains,
    },
    Operation {
        name: "get",
        spec: Signature::Declared,
        summary: "Look up a key, yielding absence when it is missing.",
        purity: Purity::Pure,
        check: None,
        eval: eval_get,
    },
    Operation {
        name: "count",
        spec: Signature::Declared,
        summary: "Count occurrences of a value; without an argument, count all occurrences.",
        purity: Purity::Pure,
        check: None,
        eval: eval_count,
    },
    Operation {
        name: "sort",
        spec: Signature::Declared,
        summary: "Order a collection by its elements' own key, or by one you supply.",
        purity: Purity::Pure,
        check: None,
        eval: eval_sort,
    },
    Operation {
        name: "take",
        spec: Signature::Declared,
        summary: "The first n elements, in the elements' own order.",
        purity: Purity::Pure,
        check: None,
        eval: eval_take,
    },
    Operation {
        name: "collect",
        spec: Signature::Declared,
        summary: "Consecutive values in batches of n, in order; the last batch may be shorter.",
        purity: Purity::Pure,
        check: Some(check_collect),
        eval: eval_collect,
    },
    Operation {
        name: "emit",
        spec: Signature::Declared,
        summary: "Every member of every sequence, in order: the inverse of collect.",
        purity: Purity::Pure,
        check: Some(check_emit),
        eval: eval_emit,
    },
    Operation {
        name: "flat_map",
        spec: Signature::Declared,
        summary: "map then emit: each value to a sequence, and their members in order.",
        purity: Purity::Pure,
        check: Some(check_ordered),
        eval: eval_flat_map,
    },
    Operation {
        name: "drop",
        spec: Signature::Declared,
        summary: "Everything after the first n elements, in the input's own order.",
        purity: Purity::Pure,
        check: Some(check_drop),
        eval: eval_drop,
    },
    Operation {
        name: "filter",
        spec: Signature::Declared,
        summary: "Keep the elements a predicate accepts.",
        purity: Purity::Pure,
        check: None,
        eval: eval_filter,
    },
    Operation {
        name: "map",
        spec: Signature::Declared,
        summary: "Apply a function to every element; the result keeps the input's collection type.",
        purity: Purity::Pure,
        check: None,
        eval: eval_map,
    },
    Operation {
        name: "of_type",
        spec: Signature::Declared,
        summary: "Keep the elements of a type, narrowing the element type: filter(x => x | is(U)).",
        purity: Purity::Pure,
        check: Some(check_of_type),
        eval: eval_of_type,
    },
    Operation {
        name: "is",
        spec: Signature::Declared,
        summary: "Whether a value's runtime type is a type or one of its subtypes.",
        purity: Purity::Pure,
        check: Some(check_of_type),
        eval: eval_is,
    },
    Operation {
        name: "somes",
        spec: Signature::Declared,
        summary: "Keep the present values of a collection of options, dropping the absent ones.",
        purity: Purity::Pure,
        check: None,
        eval: eval_somes,
    },
    Operation {
        name: "group",
        spec: Signature::Declared,
        summary: "Partition occurrences by key into a set of groups, each a key and its items.",
        purity: Purity::Pure,
        check: None,
        eval: eval_group,
    },
    Operation {
        name: "reduce",
        spec: Signature::Declared,
        summary: "Fold a sequence from the left, from an initial value or from its first element.",
        purity: Purity::Pure,
        check: Some(check_reduce),
        eval: eval_reduce,
    },
    Operation {
        name: "sum",
        spec: Signature::Declared,
        summary: "Add every occurrence: reduce(0, +). An empty input sums to zero.",
        purity: Purity::Pure,
        check: None,
        eval: eval_sum,
    },
    Operation {
        name: "product",
        spec: Signature::Declared,
        summary: "Multiply every occurrence: reduce(1, *). An empty input multiplies to one.",
        purity: Purity::Pure,
        check: None,
        eval: eval_product,
    },
    Operation {
        name: "average",
        spec: Signature::Declared,
        summary: "The mean of the occurrences, as a Float. Absent over an empty input.",
        purity: Purity::Pure,
        check: None,
        eval: eval_average,
    },
    Operation {
        name: "merge_all",
        spec: Signature::Declared,
        summary: "Merge every map in order, a later key winning: reduce({}, merge).",
        purity: Purity::Pure,
        check: Some(check_all),
        eval: eval_merge_all,
    },
    Operation {
        name: "union_all",
        spec: Signature::Declared,
        summary: "Every member of every set: reduce(Set([]), union).",
        purity: Purity::Pure,
        check: Some(check_all),
        eval: eval_union_all,
    },
    Operation {
        name: "concat_all",
        spec: Signature::Declared,
        summary: "Every member of every sequence, in order: reduce([], concat).",
        purity: Purity::Pure,
        check: Some(check_ordered),
        eval: eval_concat_all,
    },
    Operation {
        name: "min",
        spec: Signature::Declared,
        summary: "The least occurrence by value order: reduce(min). Absent over an empty input.",
        purity: Purity::Pure,
        check: None,
        eval: eval_min,
    },
    Operation {
        name: "max",
        spec: Signature::Declared,
        summary: "The greatest occurrence by value order: reduce(max). Absent over an empty input.",
        purity: Purity::Pure,
        check: None,
        eval: eval_max,
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
        Some(argument) if function_arity(argument) == Some(2) => {
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
                keyed.push((key, element));
            }
            if keyed.iter().all(|(key, _)| key.order_key().is_some()) {
                keyed.sort_by(|left, right| compare(&left.0.order_key(), &right.0.order_key()));
            } else {
                sort_through(cx, &mut keyed, &span, |pair| &pair.0)?;
            }
            elements = keyed.into_iter().map(|(_, element)| element).collect();
        }
        None => {
            if elements.iter().all(|element| element.order_key().is_some()) {
                elements.sort_by(|left, right| compare(&left.order_key(), &right.order_key()));
            } else {
                sort_through(cx, &mut elements, &span, |element| element)?;
            }
        }
    }
    Ok(Value::list(elements, element_type(&input)))
}

/// A stable sort by an order only the evaluator can decide, since it may
/// come from an implementation of `Ord`. Insertion sort keeps equal keys in
/// their positions and never hides a failure inside a sorting callback.
fn sort_through<T>(
    cx: &mut dyn EvalCx,
    items: &mut [T],
    span: &Range<usize>,
    key: impl Fn(&T) -> &Value,
) -> Result<(), Diagnostic> {
    for index in 1..items.len() {
        let mut position = index;
        while position > 0 {
            let order =
                cx.compare_values(key(&items[position - 1]), key(&items[position]), span)?;
            if order != Ordering::Greater {
                break;
            }
            items.swap(position - 1, position);
            position -= 1;
        }
    }
    Ok(())
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
    if let Some(traversal) = pending(&input) {
        let source = Source::Take {
            input: Arc::clone(traversal),
            n: count,
        };
        let element = traversal.element.clone();
        if traversal.finite {
            return Ok(wrapped(traversal, element, source));
        }
        // A bounded prefix of a stream is finite, so it is materialized.
        let prefix = Traversal::new(element.clone(), true, source);
        let mut values = Vec::new();
        cx.traverse(&prefix, &mut |_, value| {
            values.push(value);
            Ok(true)
        })?;
        return Ok(Value::list(values, element));
    }
    if let Value::Ranking(ranking) = &input {
        // Already ordered by score, and that order is the point.
        return Ok(Value::Ranking(Arc::new(Ranking {
            hits: ranking.hits.iter().take(count).cloned().collect(),
            element: ranking.element.clone(),
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

fn eval_collect(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "n").ok_or_else(|| wrong("collect", "a batch size", &span))?;
    let Value::Int(n) = cx.evaluate(&argument.value)? else {
        return Err(wrong("collect", "an Int batch size", &span));
    };
    let n = usize::try_from(n).ok().filter(|n| *n > 0).ok_or_else(|| {
        Diagnostic::runtime(
            argument.value.span.clone(),
            "a batch holds at least one value: `collect(n)` needs n of 1 or more",
        )
    })?;
    if let Some(traversal) = pending(&input)
        && !traversal.finite
    {
        let element = TypeRef::list(traversal.element.clone());
        let source = Source::Collect {
            input: Arc::clone(traversal),
            n,
        };
        return Ok(wrapped(traversal, element, source));
    }
    let element = element_type(&input);
    let batches = elements(&input, "collect", &span)?
        .chunks(n)
        .map(|batch| Value::list(batch.to_vec(), element.clone()))
        .collect();
    Ok(Value::list(batches, TypeRef::list(element)))
}

fn eval_emit(
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Some(traversal) = pending(&input)
        && !traversal.finite
    {
        let element = traversal.element.iterated().unwrap_or(TypeRef::DATA);
        let source = Source::Emit {
            input: Arc::clone(traversal),
        };
        return Ok(wrapped(traversal, element, source));
    }
    emitted(cx, &input, &span)
}

/// Every member of every batch of a finite input, in order.
fn emitted(cx: &mut dyn EvalCx, input: &Value, span: &Range<usize>) -> Result<Value, Diagnostic> {
    let element = element_type(input).iterated().unwrap_or(TypeRef::DATA);
    let mut members = Vec::new();
    for batch in elements(input, "emit", span)? {
        match &batch {
            // A batch an iterating function produced is traversed in place.
            Value::Traversal(inner) if inner.forced().is_none() => {
                cx.traverse(inner, &mut |_, member| {
                    members.push(member);
                    Ok(true)
                })?;
            }
            _ => members.extend(elements(&batch, "emit", span)?),
        }
    }
    Ok(Value::list(members, element))
}

fn eval_flat_map(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let mapped = eval_map(cx, input, arguments, span.clone())?;
    eval_emit(cx, mapped, &[], span)
}

fn eval_drop(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "n").ok_or_else(|| wrong("drop", "a count: `drop(5)`", &span))?;
    let Value::Int(count) = cx.evaluate(&argument.value)? else {
        return Err(wrong("drop", "an Int count", &span));
    };
    let count = usize::try_from(count).map_err(|_| {
        Diagnostic::runtime(argument.value.span.clone(), "a count cannot be negative")
    })?;
    if let Some(traversal) = pending(&input) {
        let source = Source::Drop {
            input: Arc::clone(traversal),
            n: count,
        };
        return Ok(wrapped(traversal, traversal.element.clone(), source));
    }
    if let Value::Ranking(ranking) = &input {
        return Ok(Value::Ranking(Arc::new(Ranking {
            hits: ranking.hits.iter().skip(count).cloned().collect(),
            element: ranking.element.clone(),
            retrieval: Arc::clone(&ranking.retrieval),
        })));
    }
    let elements = elements(&input, "drop", &span)?;
    Ok(Value::list(
        elements.into_iter().skip(count).collect(),
        element_type(&input),
    ))
}

fn eval_filter(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "by").ok_or_else(|| wrong("filter", "a predicate", &span))?;
    if let Some(traversal) = pending(&input) {
        let by = cx.evaluate(&argument.value)?;
        let element = narrowed_to(argument).unwrap_or_else(|| traversal.element.clone());
        let source = Source::Filter {
            input: Arc::clone(traversal),
            by,
            span: argument.value.span.clone(),
        };
        return Ok(wrapped(traversal, element, source));
    }
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
        // Kept because each is a `wanted`, so each reads as one.
        Some(wanted) => {
            let read = kept
                .into_iter()
                .map(|element| element.interpreted(&wanted).unwrap_or(element))
                .collect();
            narrowed(&input, read, wanted)
        }
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
    if let Some(traversal) = pending(&input) {
        let by = cx.evaluate(&argument.value)?;
        let source = Source::Map {
            input: Arc::clone(traversal),
            by,
            span: argument.value.span.clone(),
        };
        return Ok(wrapped(traversal, produced(argument), source));
    }
    let mut mapped = Vec::new();
    for element in elements(&input, "map", &span)? {
        mapped.push(cx.apply_lambda(argument, element)?);
    }
    // A set maps to its image, which collapses equal results.
    Ok(match input {
        Value::Set(..) => Value::set(mapped, produced(argument)),
        _ => Value::list(mapped, produced(argument)),
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
        .filter_map(|element| element.interpreted(&wanted))
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
    Ok(Value::Bool(input.interpreted(&argument.value.ty).is_some()))
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
    let Kind::Lambda {
        parameters, body, ..
    } = &argument.value.kind
    else {
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
pub(crate) fn narrowed_to(argument: &TypedArg) -> Option<TypeRef> {
    let TypedKind::Lambda {
        parameters, body, ..
    } = &argument.value.kind
    else {
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

/// The type a function argument produces per element: a lambda written in
/// place carries its body's type, and a named function its function type,
/// whose result is what each application yields.
pub(crate) fn produced(argument: &TypedArg) -> TypeRef {
    match argument.value.ty.as_function() {
        Some((_, result)) => result.clone(),
        None => argument.value.ty.clone(),
    }
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
            element: ranking.element.clone(),
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

/// A step that keeps positions needs an input that has them.
fn check_ordered(
    _: &mut dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if !call.input.is_stream() {
        return Err(Diagnostic::typing(
            span,
            format!(
                "this step keeps positions, and {} has none; sort it first",
                call.input
            ),
        ));
    }
    Ok(())
}

/// An argument a step reserves and does not take yet, refused before any
/// signature is chosen: `collect(for = …)`, a time window that closes a batch
/// after a duration.
pub(crate) fn reserved(name: &str, arguments: &[crate::ast::Arg]) -> Result<(), Diagnostic> {
    if name == "collect"
        && let Some(window) = arguments
            .iter()
            .find(|argument| argument.name.as_deref() == Some("for"))
    {
        return Err(Diagnostic::typing(
            window.value.span.clone(),
            "a time window, `collect(for = …)`, is not implemented yet; batch by count, `collect(n)`",
        ));
    }
    Ok(())
}

fn check_collect(
    cx: &mut dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    check_ordered(cx, call, span.clone())?;
    if !call
        .arguments
        .iter()
        .any(|argument| argument.name.as_deref().is_none_or(|name| name == "n"))
    {
        return Err(Diagnostic::typing(
            span,
            "`collect` needs a batch size: `collect(n)`",
        ));
    }
    if let Some(argument) = call.arguments.first()
        && let crate::ast::Kind::Int(n) = argument.value.kind
        && n < 1
    {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            "a batch holds at least one value: `collect(n)` needs n of 1 or more",
        ));
    }
    Ok(())
}

fn check_emit(
    cx: &mut dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    check_ordered(cx, call, span.clone())?;
    if let Some(batch) = call.input.iterated()
        && !batch.is_sequence()
    {
        return Err(Diagnostic::typing(
            span,
            format!(
                "`emit` gives members in their positions, and a {batch} has none; \
                 sort each one first"
            ),
        ));
    }
    Ok(())
}

fn check_drop(
    _: &mut dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    // Skipping the first values needs an order they arrive in.
    if !call.input.is_stream() {
        return Err(Diagnostic::typing(
            span,
            format!(
                "`drop` needs values in a stable order, and {} has none; sort it first",
                call.input
            ),
        ));
    }
    Ok(())
}
fn check_of_type(
    cx: &mut dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    let element = &call.bindings["T"];
    let wanted = &call.bindings["U"];
    if !crate::types::program::may_be(element, wanted) {
        return Err(Diagnostic::typing(
            span,
            format!("no {element} can be a {wanted}, so this is always false"),
        ));
    }
    // An open tree is asked what it is at run time, so only a declared type
    // can make the answer certain.
    if element.is(wanted) && *element != TypeRef::NEVER && *element != TypeRef::DATA {
        cx.warn(crate::warnings::Warning::new(
            span,
            format!("the test for {wanted} holds for every {element}, so it is always true"),
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
    let key_type = produced(argument);
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
    _: &mut dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if call.input.is_sequence() {
        return Ok(());
    }
    // A fold whose combiner does not care about order may visit any
    // collection: its result is the same in every order.
    let commutative = call.arguments.iter().any(|argument| {
        matches!(&argument.value.kind, crate::ast::Kind::Lambda { parameters, .. }
            if parameters.first().is_some_and(|name| name == "%commuted_left"))
    });
    if commutative {
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
                .find(|argument| argument.name.is_none() && function_arity(argument).is_some())
        })
        .ok_or_else(|| wrong("reduce", "a combining function", &span))?;
    let initial = super::named(arguments, "initial").or_else(|| {
        arguments
            .iter()
            .find(|argument| argument.name.is_none() && function_arity(argument).is_none())
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

/// `merge_all` folds maps, which is order-sensitive; `union_all` folds sets,
/// which is not.
fn check_all(
    cx: &mut dyn super::CheckCx,
    call: &super::spec::CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    let element = &call.bindings["T"];
    if *element == TypeRef::NEVER {
        return Ok(());
    }
    if element.constructor == crate::types::collections::SET {
        return Ok(());
    }
    if crate::types::MapKind::of(element.constructor).is_some() || *element == TypeRef::DATA {
        return check_ordered(cx, call, span);
    }
    Err(Diagnostic::typing(
        span,
        format!("this aggregation combines maps or sets, not {element}"),
    ))
}

/// Fold a finite input with a combiner, from a first element.
fn folded(
    combiner: &str,
    input: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<Option<Value>, Diagnostic> {
    let ty = element_type(input);
    let mut values = elements(input, name, span)?.into_iter();
    let Some(mut held) = values.next() else {
        return Ok(None);
    };
    for value in values {
        held = crate::evaluation::combine(combiner, held, value, &ty, span)?;
    }
    Ok(Some(held))
}

fn eval_merge_all(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(folded("merge", &input, "merge_all", &span)?
        .unwrap_or_else(|| Value::Data(Arc::new(crate::data::Data::map()))))
}

fn eval_union_all(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let element = element_type(&input).element().unwrap_or(TypeRef::NEVER);
    Ok(folded("union", &input, "union_all", &span)?
        .unwrap_or_else(|| Value::set(Vec::new(), element)))
}

fn eval_concat_all(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let element = element_type(&input).element().unwrap_or(TypeRef::NEVER);
    let mut members = Vec::new();
    for batch in elements(&input, "concat_all", &span)? {
        members.extend(elements(&batch, "concat_all", &span)?);
    }
    Ok(Value::list(members, element))
}

fn eval_product(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let floats = element_type(&input) == TypeRef::FLOAT;
    let mut whole: i64 = 1;
    let mut real: f64 = 1.0;
    for value in elements(&input, "product", &span)? {
        match value {
            Value::Int(n) => {
                whole = whole
                    .checked_mul(n)
                    .ok_or_else(|| Diagnostic::Overflow { span: span.clone() })?;
            }
            Value::Float(x) => real *= x,
            _ => return Err(wrong("product", "Int or Float occurrences", &span)),
        }
    }
    Ok(if floats {
        Value::Float(real)
    } else {
        Value::Int(whole)
    })
}

fn eval_average(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let values = elements(&input, "average", &span)?;
    if values.is_empty() {
        return Ok(Value::Absent(TypeRef::FLOAT));
    }
    let mut total = 0.0;
    for value in &values {
        total += match value {
            #[allow(clippy::cast_precision_loss)]
            Value::Int(n) => *n as f64,
            Value::Float(x) => *x,
            _ => return Err(wrong("average", "Int or Float occurrences", &span)),
        };
    }
    #[allow(clippy::cast_precision_loss)]
    let mean = total / values.len() as f64;
    Ok(Value::Present(Arc::new(Value::Float(mean))))
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
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
    wanted: Ordering,
) -> Result<Value, Diagnostic> {
    let mut best: Option<Value> = None;
    for value in elements(&input, "min", &span)? {
        best = Some(match best {
            Some(held) if cx.compare_values(&value, &held, &span)? != wanted => held,
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

/// How many parameters a checked argument takes, when it is a lambda or a
/// function value.
pub(crate) fn arity_of(argument: &TypedArg) -> Option<usize> {
    function_arity(argument)
}

fn function_arity(argument: &TypedArg) -> Option<usize> {
    match &argument.value.kind {
        TypedKind::Lambda { parameters, .. } => Some(parameters.len()),
        _ => argument
            .value
            .ty
            .as_function()
            .map(|(parameters, _)| parameters.len()),
    }
}
