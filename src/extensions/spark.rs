//! The module `io.db.spark`: the `Spark` transport and its constructor.
//!
//! `spark(...)` is the `spark` kind of a configured endpoint written in a
//! program: it takes the kind's keys and gives an endpoint that `source`,
//! `tap` and `target` read and write with the reader and the writer the
//! kind has. Constructing one opens no session. See
//! [HQL-0047](../../doc/proposals/HQL-0047/README.md),
//! [HQL-0048](../../doc/proposals/HQL-0048/README.md) and
//! [HQL-0051](../../doc/proposals/HQL-0051/README.md).

use super::Signature;
use super::{EvalCx, Operation, Purity};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::io::Endpoint;
use crate::types::Value;
use std::ops::Range;
use std::sync::Arc;

/// Every operation `io.db.spark` provides.
pub(crate) static STEPS: &[Operation] = &[Operation {
    name: "spark",
    spec: Signature::Declared,
    purity: Purity::Pure,
    shape: super::Shape::Barrier,
    check: None,
    eval: eval_spark,
}];

/// A named text argument, where it is written and not empty.
fn text(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    name: &str,
    span: &Range<usize>,
) -> Result<Option<String>, Diagnostic> {
    let Some(argument) = super::named(arguments, name) else {
        return Ok(None);
    };
    match cx.evaluate(&argument.value)? {
        Value::Str(text) if text.is_empty() => Ok(None),
        Value::Str(text) => Ok(Some(text.to_string())),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` of `spark` is a String, not {}", other.type_of()),
        )),
    }
}

fn eval_spark(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    // Every parameter is named: the declaration opens with `*`.
    drop(input);
    let refused = |message: String| Diagnostic::runtime(span.clone(), message);
    let table = text(cx, arguments, "table", &span)?
        .ok_or_else(|| refused("`spark` names its table".to_owned()))?;
    let server = crate::endpoints::nats_address(
        text(cx, arguments, "url", &span)?,
        text(cx, arguments, "url_env", &span)?,
        "spark",
    )
    .map_err(refused)?;
    let token_env = text(cx, arguments, "token_env", &span)?;
    let order = match super::named(arguments, "order") {
        None => Vec::new(),
        Some(argument) => cx
            .evaluate(&argument.value)?
            .elements()
            .unwrap_or_default()
            .into_iter()
            .map(|column| match column {
                Value::Str(name) => Ok(name.to_string()),
                other => Err(refused(format!(
                    "`order` lists column names, not {}",
                    other.type_of()
                ))),
            })
            .collect::<Result<_, _>>()?,
    };
    Ok(Value::Endpoint(Arc::new(Endpoint::Spark {
        server,
        token_env,
        table,
        order,
    })))
}
