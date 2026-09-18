//! The `semantic` extension: ranking a corpus against a query.
//!
//! Retrieval is an extension capability rather than a peer domain. It mints no
//! type of its own — `Ranking` and `Hit` are the core's — and it contributes
//! scores, which is the whole of what an index is allowed to decide.

use super::collections::{elements, wrong};
use super::{CheckCx, EvalCx, Purity, Step, collection, named_or_first};
use crate::ast::Arg;
use crate::diagnostics::Diagnostic;
use crate::document::Card;
use crate::search;
use crate::types::Type;
use crate::values::Value;
use std::ops::Range;
use std::rc::Rc;

/// Every step the `semantic` extension provides.
pub(crate) static STEPS: &[Step] = &[Step {
    name: "semantic",
    signature: "cards | semantic(\"bearer token authorization\")",
    summary: "Rank a corpus against a query, retaining the retrieval behind every score.",
    purity: Purity::Pure,
    check: check_semantic,
    eval: eval_semantic,
}];

fn check_semantic(
    cx: &mut dyn CheckCx,
    input: &Type,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Type, Diagnostic> {
    let element = collection(input, "semantic", &span)?;
    if !element.is(&Type::Doc) {
        return Err(Diagnostic::typing(
            span.clone(),
            format!("`semantic` ranks documents, not {element}"),
        ));
    }
    let argument = named_or_first(arguments, "query").ok_or_else(|| {
        Diagnostic::typing(span.clone(), "`semantic` needs a query: `semantic(\"…\")`")
    })?;
    let query = cx.infer(&argument.value)?;
    if query != Type::Str {
        return Err(Diagnostic::typing(
            argument.value.span.clone(),
            format!("a query is text, not {query}"),
        ));
    }
    Ok(Type::Ranking(Box::new(Type::Card)))
}

fn eval_semantic(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[Arg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument =
        named_or_first(arguments, "query").ok_or_else(|| wrong("semantic", "a query", &span))?;
    let Value::Str(query) = cx.evaluate(&argument.value)? else {
        return Err(wrong("semantic", "a text query", &span));
    };
    let elements = elements(&input, "semantic", &span)?;
    let cards: Vec<Rc<Card>> = elements.iter().filter_map(Value::as_card).collect();
    if cards.len() != elements.len() {
        return Err(wrong("semantic", "a collection of cards", &span));
    }
    let index = cx.vault().index_name();
    Ok(Value::Ranking(Rc::new(search::rank(
        &cards, &query, &index,
    ))))
}
