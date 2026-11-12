//! The `io.msg.nats` module: the `Nats` transport and its constructor.
//!
//! `nats(...)` is the `nats` kind of a configured endpoint written in a
//! program: it takes the kind's keys, checks them as the configuration
//! does, and gives an endpoint that `tap`, `sink` and `target` read and
//! write with the reader and the publisher of [`crate::nats`]. Constructing
//! one opens no connection. See
//! [HQL-0047](../../doc/proposals/HQL-0047/README.md) and
//! [HQL-0048](../../doc/proposals/HQL-0048/README.md).

use super::Signature;
use super::spec::CheckedCall;
use super::{CheckCx, EvalCx, Operation, Purity};
use crate::diagnostics::Diagnostic;
use crate::endpoints::{Subject, nats_address};
use crate::execution::Arg as TypedArg;
use crate::io::{Endpoint, Policy};
use crate::types::Value;
use std::ops::Range;
use std::sync::Arc;

/// Every operation the module provides.
pub(crate) static STEPS: &[Operation] = &[Operation {
    name: "nats",
    spec: Signature::Declared,
    purity: Purity::Pure,
    shape: super::Shape::Barrier,
    check: Some(refuse_in_documents),
    eval: eval_nats,
}];

/// A document reaches no network: the transport is refused where it is
/// constructed, so no use of it is left to refuse.
fn refuse_in_documents(
    _: &mut dyn CheckCx,
    _: &CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if matches!(crate::io::policy(), Policy::Document { .. }) {
        return Err(Diagnostic::typing(
            span,
            "a document reads and writes nothing on the network",
        ));
    }
    Ok(())
}

/// The parameters after the subject, in the order they are declared.
const PARAMETERS: [&str; 6] = ["stream", "url", "url_env", "from", "idle_ms", "consumer"];

fn eval_nats(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let refused = |message: String| Diagnostic::runtime(span.clone(), message);
    let Value::Str(subject) = input else {
        return Err(refused(format!(
            "a subject is a String, not {}",
            input.type_of()
        )));
    };
    let unnamed: Vec<&TypedArg> = arguments
        .iter()
        .filter(|argument| argument.name.is_none())
        .collect();
    let mut texts: [Option<String>; 6] = Default::default();
    let mut idle_ms = None;
    for (place, name) in PARAMETERS.iter().enumerate() {
        let Some(argument) = super::named(arguments, name).or_else(|| unnamed.get(place).copied())
        else {
            continue;
        };
        match cx.evaluate(&argument.value)? {
            // A parameter left at its default is one not written.
            Value::Str(text) if text.is_empty() => {}
            Value::Str(text) => texts[place] = Some(text.to_string()),
            Value::Int(0) if *name == "idle_ms" => {}
            Value::Int(count) if *name == "idle_ms" => idle_ms = Some(count),
            other => {
                return Err(refused(format!(
                    "`{name}` of `nats` is not {}",
                    other.type_of()
                )));
            }
        }
    }
    let [stream, url, url_env, from, _, consumer] = texts;
    let stream = stream.ok_or_else(|| refused("`nats` names its `stream`".to_owned()))?;
    let server = nats_address(url, url_env).map_err(refused)?;
    let held = Subject::new(
        stream,
        subject.to_string(),
        from.as_deref(),
        idle_ms,
        consumer,
    )
    .map_err(refused)?;
    Ok(Value::Endpoint(Arc::new(Endpoint::Nats { server, held })))
}
