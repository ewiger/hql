//! The core's collection steps.
//!
//! These stay in the core rather than becoming an extension because they
//! operate on the core's own collection types and mention nothing above them.

use super::{CheckCx, EvalCx, Purity, Step, collection, named_or_first};
use crate::ast::{Arg, Kind};
use crate::diagnostics::Diagnostic;
use crate::search::Ranking;
use crate::types::{self, TypeRef};
use crate::types::{Key, Value};
use std::cmp::Ordering;
use std::ops::Range;
use std::rc::Rc;

/// Every step the core provides, in the order help prints them.
pub(crate) static STEPS: &[Step] = &[
    Step {
        name: "size",
        signature: "collection | size",
        summary: "Count all element occurrences.",
        purity: Purity::Pure,
        check: check_size,
        eval: eval_size,
    },
    Step {
        name: "contains",
        signature: "collection | contains(value)",
        summary: "Whether a value occurs in the collection.",
        purity: Purity::Pure,
        check: check_contains,
        eval: eval_contains,
    },
    Step {
        name: "get",
        signature: "map | get(key)",
        summary: "Look up a key, yielding absence when it is missing.",
        purity: Purity::Pure,
        check: check_get,
        eval: eval_get,
    },
    Step {
        name: "count",
        signature: "collection | count(value)",
        summary: "Count occurrences of a value; without an argument, count all occurrences.",
        purity: Purity::Pure,
        check: check_count,
        eval: eval_count,
    },
    Step {
        name: "sort",
        signature: "collection | sort  |  collection | sort(by = c => c.title)",
        summary: "Order a collection by its elements' own key, or by one you supply.",
        purity: Purity::Pure,
        check: check_sort,
        eval: eval_sort,
    },
    Step {
        name: "take",
        signature: "collection | take(5)",
        summary: "The first n elements, in the elements' own order.",
        purity: Purity::Pure,
        check: check_take,
        eval: eval_take,
    },
    Step {
        name: "filter",
        signature: "collection | filter(c => c.kind)",
        summary: "Keep the elements a predicate accepts.",
        purity: Purity::Pure,
        check: check_filter,
        eval: eval_filter,
    },
    Step {
        name: "map",
        signature: "collection | map(c => c.title)",
        summary: "Apply a function to every element, yielding a sequence.",
        purity: Purity::Pure,
        check: check_map,
        eval: eval_map,
    },
    Step {
        name: "typed",
        signature: "cards | typed(RelationCard)",
        summary: "Keep the elements that are of a type, checked rather than trusted.",
        purity: Purity::Pure,
        check: check_typed,
        eval: eval_typed,
    },
];

fn check_count(
    cx: &mut dyn CheckCx,
    input: &TypeRef,
    args: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let element = collection(input, "count", &span)?;
    if args.len() > 1 || args.iter().any(|arg| arg.name.is_some()) {
        return Err(Diagnostic::typing(
            span,
            "count accepts one positional value",
        ));
    }
    if let Some(arg) = args.first() {
        let wanted = cx.infer(&arg.value)?;
        if element != TypeRef::NEVER && !wanted.is(&element) {
            return Err(Diagnostic::typing(
                arg.value.span.clone(),
                format!("{wanted} is not an element of {input}"),
            ));
        }
    }
    Ok(TypeRef::INT)
}

fn eval_count(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[Arg],
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

fn check_size(
    cx: &mut dyn CheckCx,
    input: &TypeRef,
    args: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    if !args.is_empty() {
        return Err(Diagnostic::typing(
            span,
            "size takes no arguments after its input",
        ));
    }
    check_count(cx, input, args, span)
}

fn eval_size(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    eval_count(cx, input, args, span)
}

fn check_contains(
    cx: &mut dyn CheckCx,
    input: &TypeRef,
    args: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    if args.len() != 1 {
        return Err(Diagnostic::typing(span, "contains needs one value"));
    }
    check_count(cx, input, args, span)?;
    Ok(TypeRef::BOOL)
}

fn eval_contains(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let result = eval_count(cx, input, args, span)?;
    Ok(Value::Bool(
        matches!(result, Value::Int(count) if count > 0),
    ))
}

fn check_get(
    cx: &mut dyn CheckCx,
    input: &TypeRef,
    args: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    if crate::types::MapKind::of(input.constructor).is_none() {
        return Err(Diagnostic::typing(
            span,
            format!("get needs a map, not {input}"),
        ));
    }
    let [argument] = args else {
        return Err(Diagnostic::typing(span, "get needs one key"));
    };
    if argument.name.is_some() {
        return Err(Diagnostic::typing(span, "get needs a positional key"));
    }
    let actual = cx.infer(&argument.value)?;
    let [key, value] = input.args.as_slice() else {
        return Err(Diagnostic::typing(span, "a map needs key and value types"));
    };
    if *key != TypeRef::NEVER && !actual.is(key) {
        return Err(Diagnostic::typing(
            span,
            format!("{actual} is not a {key} key"),
        ));
    }
    Ok(TypeRef::optional(value.clone()))
}

fn eval_get(
    cx: &mut dyn EvalCx,
    input: Value,
    args: &[Arg],
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

fn check_sort(
    cx: &mut dyn CheckCx,
    input: &TypeRef,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let element = collection(input, "sort", &span)?;
    if arguments.len() > 1
        || arguments
            .iter()
            .any(|arg| arg.name.as_deref().is_some_and(|name| name != "by"))
    {
        return Err(Diagnostic::typing(
            span,
            "sort accepts one comparison or key selector named by",
        ));
    }
    match named_or_first(arguments, "by") {
        Some(argument) if is_comparator(argument) => {
            let result = cx.comparator(argument, element.clone())?;
            if result != TypeRef::ORDERING {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    "a comparison must return Ordering",
                ));
            }
        }
        Some(argument) => {
            let key = cx.lambda(argument, element.clone())?;
            if !key.is_orderable() {
                return Err(Diagnostic::typing(
                    argument.value.span.clone(),
                    format!("a sort key must be orderable, and {key} is not"),
                ));
            }
        }
        None => require_orderable(&element, "sort", &span)?,
    }
    Ok(TypeRef::list(element))
}

fn eval_sort(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let mut elements = elements(&input, "sort", &span)?;
    match named_or_first(arguments, "by") {
        Some(argument) if is_comparator(argument) => {
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

fn is_comparator(argument: &Arg) -> bool {
    matches!(&argument.value.kind, Kind::Lambda { parameters, .. } if parameters.len() == 2)
}

fn check_take(
    cx: &mut dyn CheckCx,
    input: &TypeRef,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let element = collection(input, "take", &span)?;
    let argument = named_or_first(arguments, "n")
        .ok_or_else(|| Diagnostic::typing(span.clone(), "`take` needs a count: `take(5)`"))?;
    let count = cx.infer(&argument.value)?;
    if count != TypeRef::INT {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            format!("`take` needs an Int count, not {count}"),
        ));
    }
    // A prefix must be reproducible, which needs the elements to carry an
    // order of their own rather than the order they were discovered in.
    if !input.is_sequence() {
        require_orderable(&element, "take", &span)?;
    }
    Ok(if input.constructor.0 == "Ranking" {
        input.clone()
    } else {
        TypeRef::list(element)
    })
}

fn eval_take(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
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
        return Ok(Value::Ranking(Rc::new(Ranking {
            hits: ranking.hits.iter().take(count).cloned().collect(),
            retrieval: Rc::clone(&ranking.retrieval),
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

fn check_filter(
    cx: &mut dyn CheckCx,
    input: &TypeRef,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let element = collection(input, "filter", &span)?;
    let argument = named_or_first(arguments, "by").ok_or_else(|| {
        Diagnostic::typing(span.clone(), "`filter` needs a predicate: `filter(c => …)`")
    })?;
    let result = cx.lambda(argument, element.clone())?;
    if result != TypeRef::BOOL {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            format!("a filter predicate returns Bool, not {result}"),
        ));
    }
    Ok(input.with_element(element))
}

fn eval_filter(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
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

fn check_map(
    cx: &mut dyn CheckCx,
    input: &TypeRef,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let element = collection(input, "map", &span)?;
    let argument = named_or_first(arguments, "by")
        .ok_or_else(|| Diagnostic::typing(span.clone(), "`map` needs a function: `map(c => …)`"))?;
    // Mapping a set does not decide whether equal outputs collapse, so the
    // result is a sequence and the question stays open.
    Ok(TypeRef::list(cx.lambda(argument, element)?))
}

fn eval_map(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "by").ok_or_else(|| wrong("map", "a function", &span))?;
    let mut mapped = Vec::new();
    for element in elements(&input, "map", &span)? {
        mapped.push(cx.apply_lambda(argument, element)?);
    }
    let element = mapped
        .iter()
        .map(Value::type_of)
        .try_fold(TypeRef::NEVER, |held, next| held.common_type(&next))
        .ok_or_else(|| Diagnostic::runtime(span, "mapped values have incompatible types"))?;
    Ok(Value::list(mapped, element))
}

fn check_typed(
    _: &mut dyn CheckCx,
    input: &TypeRef,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<TypeRef, Diagnostic> {
    let element = collection(input, "typed", &span)?;
    let argument = named_or_first(arguments, "as").ok_or_else(|| {
        Diagnostic::typing(span.clone(), "`typed` needs a type: `typed(RelationCard)`")
    })?;
    let wanted = wanted_type(argument)?;
    if !wanted.is(&element) && !element.is(&wanted) {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            format!("no {element} can be a {wanted}, so this selects nothing"),
        ));
    }
    Ok(input.with_element(wanted))
}

fn eval_typed(
    _: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "as").ok_or_else(|| wrong("typed", "a type name", &span))?;
    let wanted = wanted_type(argument)?;
    // The kind was checked when the vault loaded; an authored entry alone
    // never put a card in this result.
    let kept: Vec<Value> = elements(&input, "typed", &span)?
        .into_iter()
        .filter(|element| element.type_of().is(&wanted))
        .collect();
    Ok(Value::set(kept, wanted))
}

/// The type an argument names, for `typed`.
fn wanted_type(argument: &Arg) -> Result<TypeRef, Diagnostic> {
    let Kind::Ident(name) = &argument.value.kind else {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            "`typed` takes a type name",
        ));
    };
    types::named(name, &[]).ok_or_else(|| {
        Diagnostic::name(
            argument.value.span.clone(),
            format!("unknown type `{name}`"),
        )
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
        Value::Ranking(_) => Value::list(kept, TypeRef::hit(TypeRef::CARD)),
        _ => Value::list(kept, element_type(input)),
    }
}

fn compare(left: &Option<Key>, right: &Option<Key>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.compare(right),
        _ => Ordering::Equal,
    }
}
