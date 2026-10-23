//! The core's collection steps.
//!
//! These stay in the core rather than becoming an extension because they
//! operate on the core's own collection types and mention nothing above them.

use super::Signature;
use super::{EvalCx, Operation, Purity, named_or_first};
use crate::decimal::Decimal;
use crate::diagnostics::Diagnostic;
use crate::execution::{Arg as TypedArg, Kind as TypedKind};
use crate::search::Ranking;
use crate::types::TypeRef;
use crate::types::{Key, Value};
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

use crate::traversal::{Source, Traversal};

/// A combiner: a binary operation the host provides on two values of one
/// type, with the laws the host states for it. A fold whose combiner is one
/// of these is a known fold, and a planner reads these properties to split
/// it; a program never writes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Combiner {
    /// How it is written in the `by` position: `reduce(0, +)`.
    pub name: &'static str,
    /// The values it combines.
    pub over: &'static str,
    /// The value `e` with `combine(e, x) = x = combine(x, e)`, where there
    /// is one, as HQL writes it.
    pub identity: Option<&'static str>,
    /// `combine(combine(a, b), c) = combine(a, combine(b, c))`: a fold may
    /// be computed from the folds of contiguous splits.
    pub associative: bool,
    /// `combine(a, b) = combine(b, a)`: the order of the input does not
    /// change the result, so a fold over it accepts any collection and its
    /// partial results combine in any order. A combiner that is not
    /// commutative is order-sensitive and folds only a `Seq`.
    pub commutative: bool,
}

impl Combiner {
    /// The input a fold over it accepts.
    #[must_use]
    pub fn input(&self) -> &'static str {
        if self.commutative {
            "any collection"
        } else {
            "Seq"
        }
    }
}

/// Every combiner the host provides. A Float `+` is associative only up to
/// rounding, so its partial sums are combined in input order: `sum` adds
/// from the first element to the last, and a backend that splits a Float
/// fold combines its partials in that same order.
pub static COMBINERS: &[Combiner] = &[
    Combiner {
        name: "+",
        over: "numbers",
        identity: Some("0"),
        associative: true,
        commutative: true,
    },
    Combiner {
        name: "*",
        over: "numbers",
        identity: Some("1"),
        associative: true,
        commutative: true,
    },
    Combiner {
        name: "min",
        over: "Orderable",
        identity: None,
        associative: true,
        commutative: true,
    },
    Combiner {
        name: "max",
        over: "Orderable",
        identity: None,
        associative: true,
        commutative: true,
    },
    Combiner {
        name: "union",
        over: "Set<T>",
        identity: Some("Set([])"),
        associative: true,
        commutative: true,
    },
    Combiner {
        name: "merge",
        over: "maps",
        identity: Some("{}"),
        associative: true,
        commutative: false,
    },
    Combiner {
        name: "concat",
        over: "Seq<T>",
        identity: Some("[]"),
        associative: true,
        commutative: false,
    },
];

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
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_size,
    },
    Operation {
        name: "contains",
        spec: Signature::Declared,
        summary: "Whether a value occurs in the collection.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_contains,
    },
    Operation {
        name: "join",
        spec: Signature::Declared,
        summary: "Join one other sequence: each matching pair, by keys, a predicate or both, built by `into`.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: |cx, input, arguments, span| eval_join(cx, input, arguments, span, Join::Inner),
    },
    Operation {
        name: "left_join",
        spec: Signature::Declared,
        summary: "Join one other sequence, keeping every element of the input; an unmatched one meets `none`.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: |cx, input, arguments, span| eval_join(cx, input, arguments, span, Join::Left),
    },
    Operation {
        name: "right_join",
        spec: Signature::Declared,
        summary: "Join one other sequence, keeping every element of it; an unmatched one meets `none`.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: |cx, input, arguments, span| eval_join(cx, input, arguments, span, Join::Right),
    },
    Operation {
        name: "full_join",
        spec: Signature::Declared,
        summary: "Join one other sequence, keeping every element of both; an unmatched one meets `none`.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: |cx, input, arguments, span| eval_join(cx, input, arguments, span, Join::Full),
    },
    Operation {
        name: "cross_join",
        spec: Signature::Declared,
        summary: "Every pair of the input and one other sequence, in order, built by `into`.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: |cx, input, arguments, span| eval_join(cx, input, arguments, span, Join::Cross),
    },
    Operation {
        name: "semi_join",
        spec: Signature::Declared,
        summary: "Keep each element of the input that matches some element of another sequence, once.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: |cx, input, arguments, span| eval_join(cx, input, arguments, span, Join::Semi),
    },
    Operation {
        name: "anti_join",
        spec: Signature::Declared,
        summary: "Keep each element of the input that matches no element of another sequence.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: |cx, input, arguments, span| eval_join(cx, input, arguments, span, Join::Anti),
    },
    Operation {
        name: "get",
        spec: Signature::Declared,
        summary: "Look up a key, yielding absence when it is missing.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: eval_get,
    },
    Operation {
        name: "count",
        spec: Signature::Declared,
        summary: "Count occurrences of a value; without an argument, count all occurrences.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_count,
    },
    Operation {
        name: "sort",
        spec: Signature::Declared,
        summary: "Order a collection by its elements' own key, or by one you supply.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: eval_sort,
    },
    Operation {
        name: "take",
        spec: Signature::Declared,
        summary: "The first n elements, in the elements' own order.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: eval_take,
    },
    Operation {
        name: "collect",
        spec: Signature::Declared,
        summary: "Consecutive values in batches of n, in order; the last batch may be shorter.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: Some(check_collect),
        eval: eval_collect,
    },
    Operation {
        name: "emit",
        spec: Signature::Declared,
        summary: "Every member of every sequence, in order: the inverse of collect.",
        purity: Purity::Pure,
        shape: super::Shape::Elementwise,
        check: Some(check_emit),
        eval: eval_emit,
    },
    Operation {
        name: "flat_map",
        spec: Signature::Declared,
        summary: "map then emit: each value to a sequence, and their members in order.",
        purity: Purity::Pure,
        shape: super::Shape::Elementwise,
        check: Some(check_ordered),
        eval: eval_flat_map,
    },
    Operation {
        name: "drop",
        spec: Signature::Declared,
        summary: "Everything after the first n elements, in the input's own order.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: Some(check_drop),
        eval: eval_drop,
    },
    Operation {
        name: "filter",
        spec: Signature::Declared,
        summary: "Keep the elements a predicate accepts.",
        purity: Purity::Pure,
        shape: super::Shape::Elementwise,
        check: None,
        eval: eval_filter,
    },
    Operation {
        name: "map",
        spec: Signature::Declared,
        summary: "Apply a function to every element; the result keeps the input's collection type.",
        purity: Purity::Pure,
        shape: super::Shape::Elementwise,
        check: None,
        eval: eval_map,
    },
    Operation {
        name: "of_type",
        spec: Signature::Declared,
        summary: "Keep the elements of a type, narrowing the element type: filter(x => x | is(U)).",
        purity: Purity::Pure,
        shape: super::Shape::Elementwise,
        check: Some(check_of_type),
        eval: eval_of_type,
    },
    Operation {
        name: "is",
        spec: Signature::Declared,
        summary: "Whether a value's runtime type is a type or one of its subtypes.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: Some(check_of_type),
        eval: eval_is,
    },
    Operation {
        name: "somes",
        spec: Signature::Declared,
        summary: "Keep the present values of a collection of options, dropping the absent ones.",
        purity: Purity::Pure,
        shape: super::Shape::Elementwise,
        check: None,
        eval: eval_somes,
    },
    Operation {
        name: "oks",
        spec: Signature::Declared,
        summary: "Keep the successes of a collection of results, dropping the failures.",
        purity: Purity::Pure,
        shape: super::Shape::Elementwise,
        check: None,
        eval: eval_oks,
    },
    Operation {
        name: "errs",
        spec: Signature::Declared,
        summary: "Keep the failures of a collection of results, dropping the successes.",
        purity: Purity::Pure,
        shape: super::Shape::Elementwise,
        check: None,
        eval: eval_errs,
    },
    Operation {
        name: "group",
        spec: Signature::Declared,
        summary: "Partition occurrences by key into a set of groups, each a key and its items.",
        purity: Purity::Pure,
        shape: super::Shape::Partition,
        check: None,
        eval: eval_group,
    },
    Operation {
        name: "reduce",
        spec: Signature::Declared,
        summary: "Fold a sequence from the left, from an initial value or from its first element.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: Some(check_reduce),
        eval: eval_reduce,
    },
    Operation {
        name: "sum",
        spec: Signature::Declared,
        summary: "Add every occurrence: reduce(0, +). An empty input sums to zero.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_sum,
    },
    Operation {
        name: "product",
        spec: Signature::Declared,
        summary: "Multiply every occurrence: reduce(1, *). An empty input multiplies to one.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_product,
    },
    Operation {
        name: "average",
        spec: Signature::Declared,
        summary: "The mean of the occurrences, as a Float. Absent over an empty input.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_average,
    },
    Operation {
        name: "merge_all",
        spec: Signature::Declared,
        summary: "Merge every map in order, a later key winning: reduce({:}, merge).",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: Some(check_all),
        eval: eval_merge_all,
    },
    Operation {
        name: "union_all",
        spec: Signature::Declared,
        summary: "Every member of every set: reduce(Set([]), union).",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: Some(check_all),
        eval: eval_union_all,
    },
    Operation {
        name: "concat_all",
        spec: Signature::Declared,
        summary: "Every member of every sequence, in order: reduce([], concat).",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: Some(check_ordered),
        eval: eval_concat_all,
    },
    Operation {
        name: "min",
        spec: Signature::Declared,
        summary: "The least occurrence by value order: reduce(min). Absent over an empty input.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_min,
    },
    Operation {
        name: "max",
        spec: Signature::Declared,
        summary: "The greatest occurrence by value order: reduce(max). Absent over an empty input.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_max,
    },
    Operation {
        name: "variance",
        spec: Signature::Declared,
        summary: "The population variance of the occurrences, as a Float. Absent over an \
                  empty input. Its state, a count, a mean and a sum of squared deviations, \
                  combines pairwise.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_variance,
    },
    Operation {
        name: "count_by",
        spec: Signature::Declared,
        summary: "How many occurrences have each key, as a map from key to count. Its \
                  state combines by `+` per key.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_count_by,
    },
    Operation {
        name: "histogram",
        spec: Signature::Declared,
        summary: "How many occurrences fall in each bucket of a width, keyed by the bucket's \
                  lower bound, in key order. Its state combines by `+` per bucket.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_histogram,
    },
    Operation {
        name: "top_k",
        spec: Signature::Declared,
        summary: "The k greatest occurrences, by value order or by a key, greatest first; \
                  equal keys keep input order. Two partial top-k lists combine by keeping \
                  the k greatest of both.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_top_k,
    },
    Operation {
        name: "approx_distinct",
        spec: Signature::Declared,
        summary: "An estimate of how many distinct values occur: HyperLogLog over 4096 \
                  registers and a SHA-256 hash, so every run gives the same answer. Its \
                  registers combine by `max`.",
        purity: Purity::Pure,
        shape: super::Shape::Reduction,
        check: None,
        eval: eval_approx_distinct,
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

/// Why a call of a join fits none of its forms, where the reason is one the
/// join's own rules give: no condition, which is a `cross_join`; `left`
/// without `right`; or a `Set`, which has no positions to order results by.
pub(crate) fn join_refusal(
    name: &str,
    input: &TypeRef,
    arguments: &[crate::ast::Arg],
) -> Option<String> {
    if !matches!(
        name,
        "join" | "left_join" | "right_join" | "full_join" | "semi_join" | "anti_join"
    ) {
        return None;
    }
    let written = |key: &str| {
        arguments
            .iter()
            .any(|argument| argument.name.as_deref() == Some(key))
    };
    if input.present().constructor == crate::types::collections::SET {
        return Some(format!(
            "`{name}` orders its results by position, and a Set has none; `sort` it first"
        ));
    }
    if written("left") != written("right") {
        return Some(format!(
            "`{name}` takes `left` and `right` together: a key function for each side"
        ));
    }
    if !written("left") && !written("on") && !written("by") {
        return Some(format!(
            "`{name}` states when elements match, with keys `left` and `right`, a predicate \
             `on`, or both; every pair is a `cross_join`"
        ));
    }
    None
}

/// Which elements a join keeps, and what it builds from them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Join {
    Inner,
    Left,
    Right,
    Full,
    Cross,
    Semi,
    Anti,
}

/// A join of the input with one other sequence (HQL-0013, the binary form):
/// results in the order of the left element's position, then the right's,
/// the unmatched elements of `other` that a right or full join keeps after
/// every matched result.
fn eval_join(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
    mode: Join,
) -> Result<Value, Diagnostic> {
    let lefts = elements(&input, "join", &span)?;
    let other = super::named(arguments, "other")
        .or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
        .ok_or_else(|| wrong("join", "another sequence", &span))?;
    let other = match cx.evaluate(&other.value)? {
        Value::Traversal(traversal) if traversal.forced().is_none() => {
            let mut values = Vec::new();
            cx.traverse(&traversal, &mut |_, value| {
                values.push(value);
                Ok(true)
            })?;
            Value::list(values, traversal.element.clone())
        }
        value => value,
    };
    let rights = elements(&other, "join", &span)?;
    let keys = |cx: &mut dyn EvalCx, name: &str, values: &[Value]| {
        super::named(arguments, name)
            .map(|argument| {
                values
                    .iter()
                    .map(|value| cx.apply_lambda(argument, value.clone()))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
    };
    let left_keys = keys(cx, "left", &lefts)?;
    let right_keys = keys(cx, "right", &rights)?;
    let on = super::named(arguments, "on");
    let matches = |cx: &mut dyn EvalCx, i: usize, j: usize| -> Result<bool, Diagnostic> {
        if mode == Join::Cross {
            return Ok(true);
        }
        if let (Some(left), Some(right)) = (&left_keys, &right_keys)
            && left[i] != right[j]
        {
            return Ok(false);
        }
        match on {
            Some(on) => Ok(matches!(
                cx.apply_function(on, vec![lefts[i].clone(), rights[j].clone()])?,
                Value::Bool(true)
            )),
            None => Ok(true),
        }
    };
    if matches!(mode, Join::Semi | Join::Anti) {
        let mut kept = Vec::new();
        for (i, left) in lefts.iter().enumerate() {
            let mut found = false;
            for j in 0..rights.len() {
                if matches(cx, i, j)? {
                    found = true;
                    break;
                }
            }
            if found == (mode == Join::Semi) {
                kept.push(left.clone());
            }
        }
        return Ok(Value::list(kept, element_type(&input)));
    }
    let into = super::named(arguments, "into").ok_or_else(|| wrong("join", "`into`", &span))?;
    // A side that may be missing is an Option in `into`.
    let optional_left = matches!(mode, Join::Right | Join::Full);
    let optional_right = matches!(mode, Join::Left | Join::Full);
    let present = |value: &Value, optional: bool| {
        if optional {
            Value::Present(Arc::new(value.clone()))
        } else {
            value.clone()
        }
    };
    let mut results = Vec::new();
    let mut right_matched = vec![false; rights.len()];
    for (i, left) in lefts.iter().enumerate() {
        let mut matched = false;
        for j in 0..rights.len() {
            if matches(cx, i, j)? {
                matched = true;
                right_matched[j] = true;
                results.push(cx.apply_function(
                    into,
                    vec![
                        present(left, optional_left),
                        present(&rights[j], optional_right),
                    ],
                )?);
            }
        }
        if !matched && optional_right {
            results.push(cx.apply_function(
                into,
                vec![
                    present(left, optional_left),
                    Value::Absent(element_type(&other)),
                ],
            )?);
        }
    }
    if optional_left {
        for (j, right) in rights.iter().enumerate() {
            if !right_matched[j] {
                results.push(cx.apply_function(
                    into,
                    vec![Value::Absent(element_type(&input)), present(right, true)],
                )?);
            }
        }
    }
    Ok(Value::list(results, produced(into)))
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
        // Documents are keyed by name: the first one so named, in order.
        (documents, Value::Str(key)) => {
            let found = elements(&documents, "get", &span)?
                .into_iter()
                .find(|element| element.as_card().is_some_and(|card| card.name() == &*key));
            Ok(match found {
                Some(card) => Value::Present(Arc::new(card)),
                None => Value::Absent(element_type(&documents)),
            })
        }
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

/// The type a predicate tests its parameter for: the `U` of a lambda whose
/// body is `x | is(U)` or `is(x, U)` on its own parameter, which a
/// parameter declared `T -> Is<U>` takes.
pub(crate) fn narrowing_of(argument: &crate::ast::Arg) -> Option<&str> {
    use crate::ast::Kind;
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
    if element_type(&input) == TypeRef::DECIMAL {
        let mut exact = Decimal::from_int(1);
        for value in elements(&input, "product", &span)? {
            exact = exact.mul(&decimal_of(value, "product", &span)?);
        }
        return Ok(Value::Decimal(exact));
    }
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
    if element_type(&input) == TypeRef::DECIMAL {
        if values.is_empty() {
            return Ok(Value::Absent(TypeRef::DECIMAL));
        }
        let count =
            i64::try_from(values.len()).map_err(|_| Diagnostic::Overflow { span: span.clone() })?;
        let mut total = Decimal::from_int(0);
        for value in values {
            total = total.add(&decimal_of(value, "average", &span)?);
        }
        let mean = total
            .div(&Decimal::from_int(count))
            .ok_or_else(|| Diagnostic::runtime(span.clone(), "division by zero"))?;
        return Ok(Value::Present(Arc::new(Value::Decimal(mean))));
    }
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
    if element_type(&input) == TypeRef::DECIMAL {
        let mut exact = Decimal::from_int(0);
        for value in elements(&input, "sum", &span)? {
            exact = exact.add(&decimal_of(value, "sum", &span)?);
        }
        return Ok(Value::Decimal(exact));
    }
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

/// One occurrence of a Decimal input.
fn decimal_of(value: Value, name: &str, span: &Range<usize>) -> Result<Decimal, Diagnostic> {
    match value {
        Value::Decimal(x) => Ok(x),
        Value::Int(n) => Ok(Decimal::from_int(n)),
        _ => Err(wrong(name, "Decimal occurrences", span)),
    }
}

/// The occurrences of a numeric input as Floats, an Int widened.
fn numbers(input: &Value, name: &str, span: &Range<usize>) -> Result<Vec<f64>, Diagnostic> {
    elements(input, name, span)?
        .into_iter()
        .map(|value| match value {
            #[allow(clippy::cast_precision_loss)]
            Value::Int(n) => Ok(n as f64),
            Value::Float(x) => Ok(x),
            _ => Err(wrong(name, "Int or Float occurrences", span)),
        })
        .collect()
}

/// The state `variance` folds: how many values, their running mean, and the
/// sum of squared deviations from it (Welford's). Two states combine
/// pairwise (Chan's), so the fold may be split; Float state is combined in
/// input order, as a Float `+` is.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Moments {
    count: i64,
    mean: f64,
    deviations: f64,
}

impl Moments {
    /// The state with one more value, as the longhand's lambda computes it.
    pub(crate) fn add(self, x: f64) -> Self {
        let count = self.count + 1;
        let delta = x - self.mean;
        #[allow(clippy::cast_precision_loss)]
        let mean = self.mean + delta / count as f64;
        Self {
            count,
            mean,
            deviations: self.deviations + delta * (x - mean),
        }
    }

    /// The state of two contiguous runs of input, this one first.
    // The combiner a backend that splits the fold applies to partial
    // states; `localhost` folds sequentially, and the tests hold the two equal.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn combine(self, other: Self) -> Self {
        if self.count == 0 {
            return other;
        }
        if other.count == 0 {
            return self;
        }
        let count = self.count + other.count;
        #[allow(clippy::cast_precision_loss)]
        let (left, right, whole) = (self.count as f64, other.count as f64, count as f64);
        let delta = other.mean - self.mean;
        Self {
            count,
            mean: self.mean + delta * right / whole,
            deviations: self.deviations + other.deviations + delta * delta * left * right / whole,
        }
    }

    /// The population variance, absent over no values.
    pub(crate) fn variance(self) -> Option<f64> {
        #[allow(clippy::cast_precision_loss)]
        (self.count > 0).then(|| self.deviations / self.count as f64)
    }
}

fn eval_variance(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let state = numbers(&input, "variance", &span)?
        .into_iter()
        .fold(Moments::default(), Moments::add);
    Ok(match state.variance() {
        Some(variance) => Value::Present(Arc::new(Value::Float(variance))),
        None => Value::Absent(TypeRef::FLOAT),
    })
}

/// A count per key, keys in the order first seen: the state `count_by` and
/// `histogram` fold. Two tallies combine by `+` per key.
pub(crate) type Tally = Vec<(Value, i64)>;

/// Add `n` to a key's count.
pub(crate) fn tally(counts: &mut Tally, key: Value, n: i64) {
    match counts.iter_mut().find(|(held, _)| *held == key) {
        Some((_, count)) => *count += n,
        None => counts.push((key, n)),
    }
}

/// Two tallies of contiguous runs of input as one, the left one's keys first.
// The combiner a backend that splits the fold applies to partial
// states; `localhost` folds sequentially, and the tests hold the two equal.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn combine_tallies(mut left: Tally, right: Tally) -> Tally {
    for (key, n) in right {
        tally(&mut left, key, n);
    }
    left
}

fn tally_map(
    kind: crate::types::MapKind,
    counts: Tally,
    key: TypeRef,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let (keys, counts): (Vec<Value>, Vec<Value>) = counts
        .into_iter()
        .map(|(key, count)| (key, Value::Int(count)))
        .unzip();
    crate::types::MapValue::new(kind, keys, counts, key, TypeRef::INT)
        .map(|map| Value::Map(Arc::new(map)))
        .map_err(|error| Diagnostic::runtime(span.clone(), error.to_string()))
}

fn eval_count_by(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let by = named_or_first(arguments, "by").ok_or_else(|| {
        wrong(
            "count_by",
            "a key selector: `count_by(x => x.genre)`",
            &span,
        )
    })?;
    let mut counts = Tally::new();
    for element in elements(&input, "count_by", &span)? {
        tally(&mut counts, cx.apply_lambda(by, element)?, 1);
    }
    tally_map(
        crate::types::MapKind::Unordered,
        counts,
        produced(by),
        &span,
    )
}

/// The lower bound of the bucket of `width` that holds a value: the
/// greatest multiple of `width` not above it.
fn bucket(value: &Value, width: &Value, span: &Range<usize>) -> Result<Value, Diagnostic> {
    let narrow = || {
        Diagnostic::runtime(
            span.clone(),
            "a bucket has a width above zero: `histogram(width)` needs a positive width",
        )
    };
    match (value, width) {
        (Value::Int(x), Value::Int(w)) => {
            if *w <= 0 {
                return Err(narrow());
            }
            x.checked_sub(x.rem_euclid(*w))
                .map(Value::Int)
                .ok_or_else(|| Diagnostic::Overflow { span: span.clone() })
        }
        (Value::Float(x), Value::Float(w)) => {
            if w.partial_cmp(&0.0) != Some(Ordering::Greater) || !w.is_finite() {
                return Err(narrow());
            }
            // `+ 0.0` reads a negative zero as zero, so both share a bucket.
            Ok(Value::Float((x / w).floor() * w + 0.0))
        }
        _ => Err(wrong(
            "histogram",
            "Int occurrences and an Int width, or Float occurrences and a Float width",
            span,
        )),
    }
}

fn eval_histogram(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument = named_or_first(arguments, "width")
        .ok_or_else(|| wrong("histogram", "a bucket width: `histogram(10)`", &span))?;
    let width = cx.evaluate(&argument.value)?;
    let key = width.type_of();
    let mut counts = Tally::new();
    for value in elements(&input, "histogram", &span)? {
        tally(
            &mut counts,
            bucket(&value, &width, &argument.value.span)?,
            1,
        );
    }
    tally_map(crate::types::MapKind::Sorted, counts, key, &span)
}

/// Keep `item` among the `k` greatest of `held`, which is ordered greatest
/// first: it goes after every held key not below its own, so equal keys
/// keep the order they arrived in. Inserting the items of a right partial
/// result into a left one, in order, is the combiner of two partials.
fn keep_greatest(
    cx: &mut dyn EvalCx,
    held: &mut Vec<(Value, Value)>,
    item: (Value, Value),
    k: usize,
    span: &Range<usize>,
) -> Result<(), Diagnostic> {
    let mut position = held.len();
    for (index, (key, _)) in held.iter().enumerate() {
        if cx.compare_values(key, &item.0, span)? == Ordering::Less {
            position = index;
            break;
        }
    }
    if position < k {
        held.insert(position, item);
        held.truncate(k);
    }
    Ok(())
}

fn eval_top_k(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument = named_or_first(arguments, "k")
        .ok_or_else(|| wrong("top_k", "a count: `top_k(3)`", &span))?;
    let Value::Int(k) = cx.evaluate(&argument.value)? else {
        return Err(wrong("top_k", "an Int count", &span));
    };
    let k = usize::try_from(k).map_err(|_| {
        Diagnostic::runtime(argument.value.span.clone(), "a count cannot be negative")
    })?;
    let by = super::named(arguments, "by").or_else(|| {
        arguments
            .iter()
            .filter(|argument| argument.name.is_none())
            .nth(1)
    });
    let mut held = Vec::new();
    for element in elements(&input, "top_k", &span)? {
        let key = match by {
            Some(by) => cx.apply_lambda(by, element.clone())?,
            None => element.clone(),
        };
        keep_greatest(cx, &mut held, (key, element), k, &span)?;
    }
    Ok(Value::list(
        held.into_iter().map(|(_, element)| element).collect(),
        element_type(&input),
    ))
}

/// The state `approx_distinct` folds: a HyperLogLog sketch. Each value's
/// SHA-256 digest of its canonical encoding picks a register and a run of
/// leading zeros, and a register keeps the longest run it has seen. Two
/// sketches combine by `max` per register, which is associative and
/// commutative, and the hash takes no seed, so every split and every
/// backend gives the same estimate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sketch {
    registers: Vec<u8>,
}

/// The bits of a digest that pick a register: 2^12 = 4096 registers, a
/// standard error near 1.6%.
const PRECISION: u32 = 12;

impl Default for Sketch {
    fn default() -> Self {
        Self {
            registers: vec![0; 1 << PRECISION],
        }
    }
}

impl Sketch {
    /// Record one value.
    pub(crate) fn insert(&mut self, value: &Value) {
        let hash = digest(value);
        let index = usize::try_from(hash >> (64 - PRECISION)).unwrap_or(0);
        let rest = hash << PRECISION;
        let run = (rest.leading_zeros() + 1).min(64 - PRECISION + 1);
        if let Some(register) = self.registers.get_mut(index) {
            *register = (*register).max(u8::try_from(run).unwrap_or(u8::MAX));
        }
    }

    /// The sketch of two runs of input.
    // The combiner a backend that splits the fold applies to partial
    // states; `localhost` folds sequentially, and the tests hold the two equal.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn combine(mut self, other: &Self) -> Self {
        for (mine, theirs) in self.registers.iter_mut().zip(&other.registers) {
            *mine = (*mine).max(*theirs);
        }
        self
    }

    /// The estimated number of distinct values: the harmonic mean of the
    /// registers, by linear counting while registers are still empty.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub(crate) fn estimate(&self) -> i64 {
        let m = self.registers.len() as f64;
        let alpha = 0.7213 / (1.0 + 1.079 / m);
        let harmonic: f64 = self
            .registers
            .iter()
            .map(|register| (-f64::from(*register)).exp2())
            .sum();
        let raw = alpha * m * m / harmonic;
        let empty = self
            .registers
            .iter()
            .filter(|register| **register == 0)
            .count();
        let estimate = if raw <= 2.5 * m && empty > 0 {
            m * (m / empty as f64).ln()
        } else {
            raw
        };
        estimate.round() as i64
    }
}

/// The first 64 bits of a value's SHA-256 digest.
fn digest(value: &Value) -> u64 {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    encode(value, &mut hasher);
    let bytes = hasher.finalize();
    let mut first = [0u8; 8];
    first.copy_from_slice(&bytes[..8]);
    u64::from_be_bytes(first)
}

/// Write a value's canonical encoding: two equal values write the same
/// bytes, so a set's members and a map's entries are written in the order
/// of their own digests, not the order they are held in.
fn encode(value: &Value, out: &mut sha2::Sha256) {
    use sha2::Digest;
    let text = |out: &mut sha2::Sha256, tag: &[u8], text: &str| {
        out.update(tag);
        out.update(text.len().to_le_bytes());
        out.update(text.as_bytes());
    };
    let unordered = |out: &mut sha2::Sha256, tag: &[u8], mut digests: Vec<u64>| {
        digests.sort_unstable();
        out.update(tag);
        out.update(digests.len().to_le_bytes());
        for digest in digests {
            out.update(digest.to_le_bytes());
        }
    };
    match value {
        Value::Int(n) => {
            out.update(b"i");
            out.update(n.to_le_bytes());
        }
        Value::Float(x) => {
            out.update(b"f");
            out.update((x + 0.0).to_bits().to_le_bytes());
        }
        // Equal decimals share a digest whatever their scale.
        Value::Decimal(x) => text(out, b"d", &x.canonical()),
        Value::Bool(b) => out.update([b'b', u8::from(*b)]),
        Value::Str(held) => text(out, b"s", held),
        Value::Formatted(formatted) => text(out, b"s", &formatted.text),
        Value::Absent(ty) => text(out, b"n", &ty.to_string()),
        Value::Present(held) => {
            out.update(b"p");
            encode(held, out);
        }
        Value::List(values, _) => {
            out.update(b"l");
            out.update(values.len().to_le_bytes());
            for held in values.iter() {
                encode(held, out);
            }
        }
        Value::Set(values, _) => unordered(out, b"S", values.iter().map(digest).collect()),
        Value::Map(map) => unordered(
            out,
            b"M",
            map.entries()
                .iter()
                .map(|(key, held)| digest(key) ^ digest(held).rotate_left(1))
                .collect(),
        ),
        Value::Record(record) => {
            text(out, b"r", record.ty.constructor.0);
            for (name, held) in &record.fields {
                text(out, b".", name);
                encode(held, out);
            }
        }
        Value::Group(group) => {
            out.update(b"g");
            encode(&group.key, out);
        }
        Value::Data(tree) => text(out, b"d", &format!("{tree:?}")),
        Value::Doc(doc) => text(out, b"D", &doc.path.to_string_lossy()),
        Value::Card(card) => text(out, b"c", &card.document.path.to_string_lossy()),
        other => text(out, b"o", &other.to_string()),
    }
}

fn eval_approx_distinct(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let mut sketch = Sketch::default();
    for value in elements(&input, "approx_distinct", &span)? {
        sketch.insert(&value);
    }
    Ok(Value::Int(sketch.estimate()))
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

fn eval_oks(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let success = element_type(&input)
        .as_result()
        .map_or(TypeRef::NEVER, |(value, _)| value.clone());
    let kept = elements(&input, "oks", &span)?
        .into_iter()
        .filter_map(|value| match value {
            Value::Ok(held, _) => Some(held.as_ref().clone()),
            _ => None,
        })
        .collect();
    Ok(narrowed(&input, kept, success))
}

fn eval_errs(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let failure = element_type(&input)
        .as_result()
        .map_or(TypeRef::NEVER, |(_, error)| error.clone());
    let kept = elements(&input, "errs", &span)?
        .into_iter()
        .filter_map(|value| match value {
            Value::Err(held, _) => Some(held.as_ref().clone()),
            _ => None,
        })
        .collect();
    Ok(narrowed(&input, kept, failure))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn moments(xs: &[f64]) -> Moments {
        xs.iter().copied().fold(Moments::default(), Moments::add)
    }

    #[test]
    fn variance_states_of_contiguous_splits_combine_to_the_whole() {
        let xs = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let whole = moments(&xs);
        assert_eq!(whole.variance(), Some(4.0));
        for split in 0..=xs.len() {
            let (left, right) = xs.split_at(split);
            let combined = moments(left).combine(moments(right));
            assert_eq!(combined.count, whole.count);
            let variance = combined.variance().unwrap_or(f64::NAN);
            assert!((variance - 4.0).abs() < 1e-12, "{split}: {variance}");
        }
        assert_eq!(Moments::default().variance(), None);
        assert_eq!(moments(&[3.0]).variance(), Some(0.0));
    }

    #[test]
    fn tallies_combine_by_addition_per_key() {
        let count = |words: &[&str]| {
            let mut counts = Tally::new();
            for word in words {
                tally(&mut counts, Value::Str(Arc::from(*word)), 1);
            }
            counts
        };
        let words = ["owl", "raven", "owl", "robin", "owl", "raven"];
        let whole = count(&words);
        let by_key = |entry: &(Value, i64)| entry.0.to_string();
        for split in 0..=words.len() {
            let (left, right) = words.split_at(split);
            let mut combined = combine_tallies(count(left), count(right));
            let mut expected = whole.clone();
            combined.sort_by_key(by_key);
            expected.sort_by_key(by_key);
            assert_eq!(combined, expected, "{split}");
        }
    }

    #[test]
    fn sketches_combine_by_max_and_ignore_order_and_repeats() {
        let sketch = |values: &[i64]| {
            let mut sketch = Sketch::default();
            for value in values {
                sketch.insert(&Value::Int(*value));
            }
            sketch
        };
        let values: Vec<i64> = (0..500).map(|n| n % 120).collect();
        let whole = sketch(&values);
        for split in [0, 1, 250, 499, 500] {
            let (left, right) = values.split_at(split);
            assert_eq!(sketch(left).combine(&sketch(right)), whole, "{split}");
            assert_eq!(sketch(right).combine(&sketch(left)), whole, "{split}");
        }
        let mut reversed = values.clone();
        reversed.reverse();
        assert_eq!(sketch(&reversed), whole);
        assert_eq!(Sketch::default().estimate(), 0);
        assert_eq!(sketch(&[7, 7, 7]).estimate(), 1);
        // Approximate, and the same on every run: 120 distinct values read as 121.
        assert_eq!(whole.estimate(), 121);
    }

    #[test]
    fn equal_values_encode_alike() {
        let set = |values: Vec<Value>| Value::set(values, TypeRef::INT);
        assert_eq!(
            digest(&set(vec![Value::Int(1), Value::Int(2)])),
            digest(&set(vec![Value::Int(2), Value::Int(1)]))
        );
        assert_eq!(digest(&Value::Float(0.0)), digest(&Value::Float(-0.0)));
        assert_ne!(digest(&Value::Int(1)), digest(&Value::Float(1.0)));
        assert_ne!(
            digest(&Value::list(
                vec![Value::Int(1), Value::Int(2)],
                TypeRef::INT
            )),
            digest(&Value::list(
                vec![Value::Int(2), Value::Int(1)],
                TypeRef::INT
            ))
        );
    }
}
