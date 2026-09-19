//! The `present` extension: how a value is written out.
//!
//! Rendering is terminal — nothing downstream reads a presentation — and it is
//! not the core's business either, so it lives here beside the graph layer.

use super::spec::{Output as O, StepSpec, TypePattern as P};
use super::{EvalCx, Purity, Step};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{Presentation, Value};
use std::ops::Range;
use std::sync::Arc;

/// Every step the `present` extension provides.
pub(crate) static STEPS: &[Step] = &[
    Step {
        name: "table",
        spec: StepSpec {
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
    Step {
        name: "json",
        spec: StepSpec {
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
    Step {
        name: "text",
        spec: StepSpec {
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
];

/// Every presenter accepts every value: presenting is where a type stops
/// mattering, which is why nothing downstream may read the result.
fn eval_table(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(present("table", input.to_table()))
}

fn eval_json(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(present(
        "json",
        serde_json::to_string_pretty(&input.to_json()).unwrap_or_else(|_| "null".to_owned()),
    ))
}

fn eval_text(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(present("text", input.to_string()))
}

fn present(presenter: &str, rendered: String) -> Value {
    Value::Presentation(Arc::new(Presentation {
        presenter: presenter.to_owned(),
        text: rendered,
    }))
}
