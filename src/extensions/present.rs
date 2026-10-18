//! The `present` extension: how a value is written out.
//!
//! Rendering is terminal — nothing downstream reads a presentation — and it is
//! not the core's business either, so it lives here beside the graph layer.

use super::spec::{OperationSpec, Output as O, TypePattern as P};
use super::{EvalCx, Operation, Purity};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{Presentation, Value};
use std::ops::Range;
use std::sync::Arc;

/// Every step the `present` extension provides.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "table",
        spec: OperationSpec {
            parameters: &[],
            input: P::Any,
            arguments: &[],
            output: O::Type(P::Type(crate::types::TypeConstructor("Presentation"))),
        },
        summary: "Render as a table. Terminal: nothing downstream reads it.",
        purity: Purity::Pure,
        check: None,
        eval: eval_table,
    },
    Operation {
        name: "json",
        spec: OperationSpec {
            parameters: &[],
            input: P::Any,
            arguments: &[],
            output: O::Type(P::Type(crate::types::TypeConstructor("Presentation"))),
        },
        summary: "Render as JSON. Terminal.",
        purity: Purity::Pure,
        check: None,
        eval: eval_json,
    },
    Operation {
        name: "text",
        spec: OperationSpec {
            parameters: &[],
            input: P::Any,
            arguments: &[],
            output: O::Type(P::Type(crate::types::TypeConstructor("Presentation"))),
        },
        summary: "Render as plain text. Terminal.",
        purity: Purity::Pure,
        check: None,
        eval: eval_text,
    },
    Operation {
        name: "markdown",
        spec: OperationSpec {
            parameters: &[],
            input: P::Any,
            arguments: &[],
            output: O::Type(P::Type(crate::types::TypeConstructor("Presentation"))),
        },
        summary: "Render as Markdown: a record as a table, a sequence as a list. Terminal.",
        purity: Purity::Pure,
        check: None,
        eval: eval_markdown,
    },
];

/// Every presenter accepts every value: presenting is where a type stops
/// mattering, which is why nothing downstream may read the result.
fn eval_table(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(present(
        "table",
        input.to_table(),
        input.to_markdown_table(),
    ))
}

fn eval_json(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    let json = serde_json::to_string_pretty(&input.to_json()).unwrap_or_else(|_| "null".to_owned());
    let markdown = format!("```json\n{json}\n```");
    Ok(present("json", json, markdown))
}

fn eval_text(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    let text = input.to_string();
    Ok(present("text", text.clone(), text))
}

fn eval_markdown(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    let markdown = input.to_markdown();
    Ok(present("markdown", markdown.clone(), markdown))
}

/// A presentation carries its text and its document form, so a host that
/// writes into a document need not re-render a value it no longer has.
fn present(presenter: &str, text: String, markdown: String) -> Value {
    Value::Presentation(Arc::new(Presentation {
        presenter: presenter.to_owned(),
        text,
        markdown,
    }))
}
