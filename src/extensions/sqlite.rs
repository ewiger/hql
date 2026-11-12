//! The module `io.db.sqlite`: the `Sqlite` transport and its constructor.
//!
//! `sqlite(...)` is the `sqlite` kind of a configured endpoint written in a
//! program: it takes the kind's keys and gives an endpoint that `source`,
//! `tap` and `target` read and write with the reader and the writer the
//! kind has. Constructing one opens nothing. See
//! [HQL-0047](../../doc/proposals/HQL-0047/README.md) and
//! [HQL-0048](../../doc/proposals/HQL-0048/README.md).

use super::Signature;
use super::{EvalCx, Operation, Purity};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::io::Endpoint;
use crate::types::Value;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

/// Every operation `io.db.sqlite` provides.
pub(crate) static STEPS: &[Operation] = &[Operation {
    name: "sqlite",
    spec: Signature::Declared,
    purity: Purity::Pure,
    shape: super::Shape::Barrier,
    check: None,
    eval: eval_sqlite,
}];

fn text(value: Value, what: &str, span: &Range<usize>) -> Result<String, Diagnostic> {
    match value {
        Value::Str(text) if !text.is_empty() => Ok(text.to_string()),
        Value::Str(_) => Err(Diagnostic::runtime(
            span.clone(),
            format!("`sqlite` names its {what}"),
        )),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!(
                "the {what} of `sqlite` is a String, not {}",
                other.type_of()
            ),
        )),
    }
}

fn eval_sqlite(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let path = text(input, "path", &span)?;
    let unnamed: Vec<&TypedArg> = arguments
        .iter()
        .filter(|argument| argument.name.is_none())
        .collect();
    let table = match super::named(arguments, "table").or_else(|| unnamed.first().copied()) {
        Some(argument) => text(cx.evaluate(&argument.value)?, "table", &span)?,
        None => return Err(Diagnostic::runtime(span, "`sqlite` names its table")),
    };
    let order = match super::named(arguments, "order").or_else(|| unnamed.get(1).copied()) {
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
    Ok(Value::Endpoint(Arc::new(Endpoint::Sqlite {
        path: PathBuf::from(path),
        table,
        order,
    })))
}
