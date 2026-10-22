//! The `net` extension: endpoints that are transports, read and written by
//! `source`, `tap`, `target` and `sink` like any other.
//!
//! Constructing one sends nothing; the request or connection happens when an
//! `io` operation uses it, in [`crate::io`] and [`crate::net`]. A program
//! imports the extension, `import net`, so network access is visible in its
//! first lines. See [HQL-0021](../../doc/proposals/HQL-0021/README.md).

use super::Signature;
use super::spec::{ArgumentSpec, CheckedCall, OperationSpec, Output as O, TypePattern as P};
use super::{CheckCx, EvalCx, Operation, Purity};
use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::io::{Endpoint, Policy};
use crate::types::{TypeConstructor, Value};
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

const STRING: P = P::Type(TypeConstructor("String"));

/// Every operation the `net` extension provides.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "http",
        spec: Signature::Native(OperationSpec {
            parameters: &[],
            input: STRING,
            arguments: &[
                ArgumentSpec {
                    required: false,
                    ..ArgumentSpec::value("method", STRING)
                },
                ArgumentSpec {
                    required: false,
                    ..ArgumentSpec::value("headers", P::Type(TypeConstructor("Data")))
                },
            ],
            output: O::Type(P::Type(TypeConstructor("Http"))),
        }),
        summary: "An HTTP resource, as an endpoint: read by a GET once per evaluation, \
                  written by one request per document.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: Some(refuse_in_documents),
        eval: eval_http,
    },
    Operation {
        name: "websocket",
        spec: Signature::Native(OperationSpec {
            parameters: &[],
            input: STRING,
            arguments: &[],
            output: O::Type(P::Type(TypeConstructor("WebSocket"))),
        }),
        summary: "A WebSocket connection, as an endpoint: one document per message.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: Some(refuse_in_documents),
        eval: eval_websocket,
    },
    Operation {
        name: "socket",
        spec: Signature::Native(OperationSpec {
            parameters: &[],
            input: STRING,
            arguments: &[ArgumentSpec::value("port", P::Type(TypeConstructor("Int")))],
            output: O::Type(P::Type(TypeConstructor("Socket"))),
        }),
        summary: "A TCP connection, as an endpoint: documents separated as their format \
                  separates them.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: Some(refuse_in_documents),
        eval: eval_socket,
    },
];

/// A document reaches no network: every network endpoint is refused where it
/// is constructed, so no use of one is left to refuse.
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

fn text(value: Value, what: &str, span: &Range<usize>) -> Result<String, Diagnostic> {
    match value {
        Value::Str(text) => Ok(text.to_string()),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("{what} is a String, not {}", other.type_of()),
        )),
    }
}

fn endpoint(endpoint: Endpoint) -> Value {
    Value::Endpoint(Arc::new(endpoint))
}

fn eval_http(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let url = text(input, "a URL", &span)?;
    let unnamed: Vec<&TypedArg> = arguments
        .iter()
        .filter(|argument| argument.name.is_none())
        .collect();
    let method = match super::named(arguments, "method").or_else(|| unnamed.first().copied()) {
        Some(argument) => text(cx.evaluate(&argument.value)?, "a method", &span)?,
        None => "GET".to_owned(),
    };
    let headers = match super::named(arguments, "headers").or_else(|| unnamed.get(1).copied()) {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Data(data) => headers(&data, &span)?,
            other => {
                return Err(Diagnostic::runtime(
                    span,
                    format!(
                        "headers are a map of names to values, not {}",
                        other.type_of()
                    ),
                ));
            }
        },
        None => BTreeMap::new(),
    };
    Ok(endpoint(Endpoint::Http {
        url,
        method: method.to_uppercase(),
        headers,
    }))
}

/// A map of header names to values, each value text.
fn headers(data: &Data, span: &Range<usize>) -> Result<BTreeMap<String, String>, Diagnostic> {
    let Data::Map(entries) = data else {
        return Err(Diagnostic::runtime(
            span.clone(),
            "headers are a map of names to values",
        ));
    };
    entries
        .iter()
        .map(|(name, value)| match value {
            Data::Str(text) => Ok((name.clone(), text.clone())),
            Data::Int(number) => Ok((name.clone(), number.to_string())),
            _ => Err(Diagnostic::runtime(
                span.clone(),
                format!("the header {name:?} is not text"),
            )),
        })
        .collect()
}

fn eval_websocket(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(endpoint(Endpoint::WebSocket(text(input, "a URL", &span)?)))
}

fn eval_socket(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let host = text(input, "a host", &span)?;
    let port = match super::named_or_first(arguments, "port") {
        Some(argument) => cx.evaluate(&argument.value)?,
        None => return Err(Diagnostic::runtime(span, "a socket needs a `port`")),
    };
    let port = match port {
        Value::Int(port) => u16::try_from(port).ok().filter(|port| *port > 0),
        _ => None,
    }
    .ok_or_else(|| Diagnostic::runtime(span.clone(), "a port is an Int from 1 to 65535"))?;
    Ok(endpoint(Endpoint::Socket(host, port)))
}
