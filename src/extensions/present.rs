//! The `present` extension: how a value is written out.
//!
//! Rendering is terminal — nothing downstream reads a presentation — and it is
//! not the core's business either, so it lives here beside the graph layer.

use super::{CheckCx, EvalCx, Purity, Step};
use crate::ast::Arg;
use crate::diagnostics::Diagnostic;
use crate::types::Type;
use crate::types::{Presentation, Value};
use std::ops::Range;
use std::rc::Rc;

/// Every step the `present` extension provides.
pub(crate) static STEPS: &[Step] = &[
    Step {
        name: "table",
        signature: "value | table",
        summary: "Render as a table. Terminal: nothing downstream reads it.",
        purity: Purity::Pure,
        check: check_presentation,
        eval: eval_table,
    },
    Step {
        name: "json",
        signature: "value | json",
        summary: "Render as JSON. Terminal.",
        purity: Purity::Pure,
        check: check_presentation,
        eval: eval_json,
    },
    Step {
        name: "text",
        signature: "value | text",
        summary: "Render as plain text. Terminal.",
        purity: Purity::Pure,
        check: check_presentation,
        eval: eval_text,
    },
];

/// Every presenter accepts every value: presenting is where a type stops
/// mattering, which is why nothing downstream may read the result.
fn check_presentation(
    _: &mut dyn CheckCx,
    _: &Type,
    _: &[Arg],
    _: Range<usize>,
) -> Result<Type, Diagnostic> {
    Ok(Type::Presentation)
}

fn eval_table(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[Arg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(present("table", input.to_table()))
}

fn eval_json(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[Arg],
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
    _: &[Arg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(present("text", input.to_string()))
}

fn present(presenter: &str, rendered: String) -> Value {
    Value::Presentation(Rc::new(Presentation {
        presenter: presenter.to_owned(),
        text: rendered,
    }))
}
