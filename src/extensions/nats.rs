//! The modules `io.msg.nats` and `io.msg.nats_jetstream`: the `Nats` and
//! `NatsJetStream` transports and their constructors.
//!
//! Each constructor is its kind of configured endpoint written in a
//! program: it takes the kind's keys, checks them as the configuration
//! does, and gives an endpoint that `tap`, `sink` and `target` read and
//! write with the readers and the publisher of [`crate::nats`].
//! Constructing one opens no connection. See
//! [HQL-0047](../../doc/proposals/HQL-0047/index.hmd) and
//! [HQL-0048](../../doc/proposals/HQL-0048/index.hmd).

use super::Signature;
use super::spec::CheckedCall;
use super::{CheckCx, EvalCx, Operation, Purity};
use crate::diagnostics::Diagnostic;
use crate::endpoints::{Subject, nats_address};
use crate::execution::Arg as TypedArg;
use crate::io::{Endpoint, Policy};
use crate::types::Value;
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

/// Every operation `io.msg.nats` provides.
pub(crate) static CORE_STEPS: &[Operation] = &[Operation {
    name: "nats",
    spec: Signature::Declared,
    purity: Purity::Pure,
    shape: super::Shape::Barrier,
    check: Some(refuse_in_documents),
    eval: eval_nats,
}];

/// Every operation `io.msg.nats_jetstream` provides.
pub(crate) static JETSTREAM_STEPS: &[Operation] = &[Operation {
    name: "nats_jetstream",
    spec: Signature::Declared,
    purity: Purity::Pure,
    shape: super::Shape::Barrier,
    check: Some(refuse_in_documents),
    eval: eval_nats_jetstream,
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

/// What a constructor was given: its subject, each text parameter written,
/// and `idle_ms`. A parameter left at its default is one not written.
struct Given {
    subject: String,
    texts: BTreeMap<&'static str, String>,
    idle_ms: Option<i64>,
}

/// Read a constructor's arguments, `parameters` being those after the
/// subject in the order they are declared.
fn given(
    cx: &mut dyn EvalCx,
    constructor: &str,
    input: Value,
    arguments: &[TypedArg],
    parameters: &[&'static str],
    span: &Range<usize>,
) -> Result<Given, Diagnostic> {
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
    let mut held = Given {
        subject: subject.to_string(),
        texts: BTreeMap::new(),
        idle_ms: None,
    };
    for (place, name) in parameters.iter().enumerate() {
        let Some(argument) = super::named(arguments, name).or_else(|| unnamed.get(place).copied())
        else {
            continue;
        };
        match cx.evaluate(&argument.value)? {
            Value::Str(text) if text.is_empty() => {}
            Value::Str(text) => {
                held.texts.insert(name, text.to_string());
            }
            Value::Int(0) if *name == "idle_ms" => {}
            Value::Int(count) if *name == "idle_ms" => held.idle_ms = Some(count),
            other => {
                return Err(refused(format!(
                    "`{name}` of `{constructor}` is not {}",
                    other.type_of()
                )));
            }
        }
    }
    Ok(held)
}

fn eval_nats(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let refused = |message: String| Diagnostic::runtime(span.clone(), message);
    let mut held = given(
        cx,
        "nats",
        input,
        arguments,
        &["url", "url_env", "idle_ms"],
        &span,
    )?;
    let server = nats_address(
        held.texts.remove("url"),
        held.texts.remove("url_env"),
        "nats",
    )
    .map_err(refused)?;
    // A core subject checks its count as a stream's subject does.
    let idle = Subject::new(String::new(), String::new(), None, held.idle_ms, None)
        .map_err(refused)?
        .idle;
    Ok(Value::Endpoint(Arc::new(Endpoint::Nats {
        server,
        subject: held.subject,
        idle,
    })))
}

fn eval_nats_jetstream(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let refused = |message: String| Diagnostic::runtime(span.clone(), message);
    let mut held = given(
        cx,
        "nats_jetstream",
        input,
        arguments,
        &["stream", "url", "url_env", "from", "idle_ms", "consumer"],
        &span,
    )?;
    let stream = held
        .texts
        .remove("stream")
        .ok_or_else(|| refused("`nats_jetstream` names its `stream`".to_owned()))?;
    let server = nats_address(
        held.texts.remove("url"),
        held.texts.remove("url_env"),
        "nats_jetstream",
    )
    .map_err(refused)?;
    let consumer = held.texts.remove("consumer");
    let subject = Subject::new(
        stream,
        held.subject,
        held.texts.get("from").map(String::as_str),
        held.idle_ms,
        consumer,
    )
    .map_err(refused)?;
    Ok(Value::Endpoint(Arc::new(Endpoint::NatsJetStream {
        server,
        held: subject,
    })))
}
