//! The module `io.db.postgres`: the `Postgres` transport and its constructor.
//!
//! `postgres(...)` is the `postgres` kind of a configured endpoint written in
//! a program: it takes the kind's keys and gives an endpoint that `source`,
//! `tap` and `target` read and write with the reader and the writer the
//! kind has. Constructing one opens no connection, and the connection
//! string is read from the environment variable `dsn_env` names where the
//! endpoint is used. See [HQL-0047](../../doc/proposals/HQL-0047/README.md),
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

/// Every operation `io.db.postgres` provides.
pub(crate) static STEPS: &[Operation] = &[Operation {
    name: "postgres",
    spec: Signature::Declared,
    purity: Purity::Pure,
    shape: super::Shape::Barrier,
    check: None,
    eval: eval_postgres,
}];

fn text(value: Value, what: &str, span: &Range<usize>) -> Result<String, Diagnostic> {
    match value {
        Value::Str(text) if !text.is_empty() => Ok(text.to_string()),
        Value::Str(_) => Err(Diagnostic::runtime(
            span.clone(),
            format!("`postgres` names its {what}"),
        )),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!(
                "the {what} of `postgres` is a String, not {}",
                other.type_of()
            ),
        )),
    }
}

fn eval_postgres(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    // Every parameter is named: the declaration opens with `*`, so the
    // call has no positional input.
    drop(input);
    let dsn_env = match super::named(arguments, "dsn_env") {
        Some(argument) => text(
            cx.evaluate(&argument.value)?,
            "`dsn_env`, the variable holding its connection string",
            &span,
        )?,
        None => {
            return Err(Diagnostic::runtime(
                span,
                "`postgres` names `dsn_env`, the variable holding its connection string",
            ));
        }
    };
    let table = match super::named(arguments, "table") {
        Some(argument) => text(cx.evaluate(&argument.value)?, "table", &span)?,
        None => return Err(Diagnostic::runtime(span, "`postgres` names its table")),
    };
    let order = match super::named(arguments, "order") {
        None => Vec::new(),
        Some(argument) => {
            let columns = cx.evaluate(&argument.value)?;
            columns
                .elements()
                .unwrap_or_default()
                .into_iter()
                .map(|column| match column {
                    Value::Str(name) => Ok(name.to_string()),
                    other => Err(Diagnostic::runtime(
                        span.clone(),
                        format!("`order` lists column names, not {}", other.type_of()),
                    )),
                })
                .collect::<Result<_, _>>()?
        }
    };
    Ok(Value::Endpoint(Arc::new(Endpoint::Postgres {
        dsn_env,
        table,
        order,
    })))
}
