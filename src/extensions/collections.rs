//! The core's collection steps.
//!
//! These stay in the core rather than becoming an extension because they
//! operate on the core's own collection types and mention nothing above them.

use super::{CheckCx, EvalCx, Purity, Step, collection, named_or_first};
use crate::ast::{Arg, Kind};
use crate::diagnostics::Diagnostic;
use crate::search::Ranking;
use crate::types::{self, Type};
use crate::types::{Key, Value};
use std::cmp::Ordering;
use std::ops::Range;
use std::rc::Rc;

/// Every step the core provides, in the order help prints them.
pub(crate) static STEPS: &[Step] = &[
    Step {
        name: "count",
        signature: "collection | count",
        summary: "How many elements a collection holds.",
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
    _: &mut dyn CheckCx,
    input: &Type,
    _: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
    collection(input, "count", &span)?;
    Ok(Type::Int)
}

fn eval_count(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(Value::Int(
        elements(&input, "count", &span)?
            .len()
            .try_into()
            .unwrap_or(i64::MAX),
    ))
}

fn check_sort(
    cx: &mut dyn CheckCx,
    input: &Type,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
    let element = collection(input, "sort", &span)?;
    match named_or_first(arguments, "by") {
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
    Ok(Type::Seq(Box::new(element)))
}

fn eval_sort(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let mut elements = elements(&input, "sort", &span)?;
    match named_or_first(arguments, "by") {
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
    Ok(Value::seq(elements, element_type(&input)))
}

fn check_take(
    cx: &mut dyn CheckCx,
    input: &Type,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
    let element = collection(input, "take", &span)?;
    let argument = named_or_first(arguments, "n")
        .ok_or_else(|| Diagnostic::typing(span.clone(), "`take` needs a count: `take(5)`"))?;
    let count = cx.infer(&argument.value)?;
    if count != Type::Int {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            format!("`take` needs an Int count, not {count}"),
        ));
    }
    // A prefix must be reproducible, which needs the elements to carry an
    // order of their own rather than the order they were discovered in.
    require_orderable(&element, "take", &span)?;
    Ok(match input {
        Type::Ranking(_) => input.clone(),
        _ => Type::Seq(Box::new(element)),
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
    Ok(Value::seq(elements, element_type(&input)))
}

fn check_filter(
    cx: &mut dyn CheckCx,
    input: &Type,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
    let element = collection(input, "filter", &span)?;
    let argument = named_or_first(arguments, "by").ok_or_else(|| {
        Diagnostic::typing(span.clone(), "`filter` needs a predicate: `filter(c => …)`")
    })?;
    let result = cx.lambda(argument, element.clone())?;
    if result != Type::Bool {
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
    input: &Type,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
    let element = collection(input, "map", &span)?;
    let argument = named_or_first(arguments, "by")
        .ok_or_else(|| Diagnostic::typing(span.clone(), "`map` needs a function: `map(c => …)`"))?;
    // Mapping a set does not decide whether equal outputs collapse, so the
    // result is a sequence and the question stays open.
    Ok(Type::Seq(Box::new(cx.lambda(argument, element)?)))
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
    let element = mapped.first().map_or(Type::Data, Value::type_of);
    Ok(Value::seq(mapped, element))
}

fn check_typed(
    _: &mut dyn CheckCx,
    input: &Type,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
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
fn wanted_type(argument: &Arg) -> Result<Type, Diagnostic> {
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

fn require_orderable(element: &Type, name: &str, span: &Range<usize>) -> Result<(), Diagnostic> {
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
pub(crate) fn element_type(input: &Value) -> Type {
    input.type_of().element().unwrap_or(Type::Data)
}

/// Keep a filtered collection in the shape it came in.
fn rebuild(input: &Value, kept: Vec<Value>) -> Value {
    match input {
        Value::Set(_, element) => Value::set(kept, element.clone()),
        Value::Ranking(_) => Value::seq(kept, Type::Hit(Box::new(Type::Card))),
        _ => Value::seq(kept, element_type(input)),
    }
}

fn compare(left: &Option<Key>, right: &Option<Key>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.compare(right),
        _ => Ordering::Equal,
    }
}
