//! The `io` extension: endpoints read by `source` and `tap`, and written by
//! `target` and `sink`.
//!
//! An endpoint says where values are and a format type how its bytes are
//! documents. Which operation is used says whether the documents are held
//! (`source`, `target`) or taken one at a time (`tap`, `sink`); which endpoint
//! says whether a file is replaced or appended to. The I/O itself is in
//! [`crate::io`]. See [HQL-0021](../../doc/proposals/HQL-0021/README.md).

use super::spec::{
    ArgumentKind as A, ArgumentSpec, CheckedCall, OperationSpec, Output as O, TypeParameter,
    TypePattern as P,
};
use super::{CheckCx, EvalCx, Operation, Purity};
use crate::ast::{Expr, Kind};
use crate::diagnostics::Diagnostic;
use crate::execution::{Arg as TypedArg, Kind as TypedKind};
use crate::io::{self, Destination, Endpoint, Policy, Reading};
use crate::traversal::{Source, Traversal};
use crate::types::{TypeConstructor, TypeRef, Value};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The host's endpoints, bound by name as `vault` is.
pub(crate) const BINDINGS: [(&str, Endpoint); 3] = [
    ("stdin", Endpoint::Stdin),
    ("stdout", Endpoint::Stdout),
    ("stderr", Endpoint::Stderr),
];

const FORMAT: ArgumentSpec = ArgumentSpec {
    name: "format",
    kind: A::TypeName("F"),
    required: false,
    positional: true,
    named: true,
};

const FORMAT_PARAMETER: TypeParameter = TypeParameter {
    name: "F",
    bound: Some(TypeConstructor("String")),
};

const ENDPOINT: ArgumentSpec =
    ArgumentSpec::value("endpoint", P::Type(TypeConstructor("Writable")));

/// Every operation the `io` extension provides.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "file",
        spec: OperationSpec {
            parameters: &[],
            input: P::Type(TypeConstructor("String")),
            arguments: &[],
            output: O::Type(P::Type(TypeConstructor("File"))),
        },
        summary: "The file at a path, as an endpoint: read whole, written by replacing it; \
                  its `.end` is appended to and followed.",
        purity: Purity::Pure,
        check: Some(check_file),
        eval: eval_file,
    },
    Operation {
        name: "source",
        spec: OperationSpec {
            parameters: &[FORMAT_PARAMETER],
            input: P::Type(TypeConstructor("Readable")),
            arguments: &[FORMAT],
            output: O::Computed("Seq<T> | Stream<T>"),
        },
        summary: "Read an endpoint's documents in a format: every one, as a Seq, when it can \
                  be read again; each as it arrives, as a Stream, when it cannot.",
        purity: Purity::Effectful,
        check: Some(check_read),
        eval: eval_source,
    },
    Operation {
        name: "tap",
        spec: OperationSpec {
            parameters: &[FORMAT_PARAMETER],
            input: P::Type(TypeConstructor("Readable")),
            arguments: &[FORMAT],
            output: O::Computed("Stream<T>"),
        },
        summary: "Read an endpoint's documents one at a time, as a Stream; at a file's \
                  `.end`, each one appended from now on.",
        purity: Purity::Effectful,
        check: Some(check_read),
        eval: eval_tap,
    },
    Operation {
        name: "target",
        spec: OperationSpec {
            parameters: &[
                TypeParameter {
                    name: "T",
                    bound: None,
                },
                FORMAT_PARAMETER,
            ],
            input: P::Var("T"),
            arguments: &[ENDPOINT, FORMAT],
            output: O::Type(P::Type(TypeConstructor("Unit"))),
        },
        summary: "Write one completed value as one document; a file is replaced, its `.end` \
                  appended to.",
        purity: Purity::Effectful,
        check: Some(check_target),
        eval: eval_target,
    },
    Operation {
        name: "sink",
        spec: OperationSpec {
            parameters: &[
                TypeParameter {
                    name: "T",
                    bound: None,
                },
                FORMAT_PARAMETER,
            ],
            input: P::Iterable(&P::Var("T")),
            arguments: &[ENDPOINT, FORMAT],
            output: O::Type(P::Type(TypeConstructor("Unit"))),
        },
        summary: "Write each value of a stream as one document, as it arrives; a file is \
                  replaced, its `.end` appended to.",
        purity: Purity::Effectful,
        check: Some(check_sink),
        eval: eval_sink,
    },
];

/// The literal path an endpoint expression names: `file("x.json")`, or its
/// `.end`.
fn literal_path(expression: &Expr) -> Option<&str> {
    match &expression.kind {
        Kind::Call { callee, arguments } => match (&callee.kind, arguments.as_slice()) {
            (Kind::Ident(name), [argument]) if name == "file" && argument.name.is_none() => {
                match &argument.value.kind {
                    Kind::Str(path) => Some(path),
                    _ => None,
                }
            }
            _ => None,
        },
        Kind::Pipe(input, step) => match (&input.kind, &step.kind) {
            (Kind::Str(path), Kind::Ident(name)) if name == "file" => Some(path),
            _ => None,
        },
        Kind::Field(inner, field) if field == "end" => literal_path(inner),
        _ => None,
    }
}

/// The endpoint expression a write names, as written.
fn written_endpoint<'a>(call: &CheckedCall<'a>) -> Option<&'a Expr> {
    call.arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some("endpoint"))
        .or_else(|| {
            call.arguments
                .iter()
                .find(|argument| argument.name.is_none())
        })
        .map(|argument| &argument.value)
}

/// The format a call reads or writes in: the one it names, or the one a
/// literal path's extension implies.
pub(crate) fn checked_format(
    call: &CheckedCall<'_>,
    endpoint: Option<&Expr>,
    verb: &str,
) -> Result<TypeConstructor, String> {
    if let Some(format) = call.bindings.get("F") {
        let format = format.constructor;
        if !crate::extensions::format::FORMATS
            .iter()
            .any(|(name, _)| *name == format.0)
        {
            return Err(format!(
                "`{verb}` reads and writes a format type such as Json, not {format}"
            ));
        }
        return Ok(format);
    }
    match endpoint.and_then(literal_path) {
        Some(path) => io::format_of_extension(Path::new(path)).ok_or_else(|| {
            format!("the extension of {path:?} implies no format; name it: `{verb}(…, Json)`")
        }),
        None => Err(format!(
            "`{verb}` needs a format, `{verb}(…, Json)`: only a literal file path implies one"
        )),
    }
}

/// The element a channel type holds, when it is one.
fn channel_element(ty: &TypeRef) -> Option<&TypeRef> {
    (ty.constructor == crate::types::builtin::CHANNEL)
        .then(|| ty.args.first())
        .flatten()
}

const NO_FORMAT: &str = "a channel holds values, not bytes, so it takes no format";

/// The type `source` or `tap` gives.
pub(crate) fn result(name: &str, call: &CheckedCall<'_>) -> Result<TypeRef, String> {
    // A channel's values are what it holds now to `source`, and what is
    // written from now on to `tap`.
    if let Some(element) = channel_element(call.input) {
        if call.bindings.contains_key("F") {
            return Err(NO_FORMAT.to_owned());
        }
        return Ok(if name == "tap" {
            crate::types::collections::STREAM.apply([element.clone()])
        } else {
            TypeRef::seq(element.clone())
        });
    }
    let format = checked_format(call, call.input_expr, name)?;
    let element = io::document_type(format)?;
    if name == "tap" {
        if !io::streams(format) {
            return Err(format!(
                "{} is one document, so it is not read one at a time; read it whole with `source`",
                format.0
            ));
        }
        return Ok(crate::types::collections::STREAM.apply([element]));
    }
    if call.input.is(&TypeRef::FILE_END) {
        return Err(
            "the end of a file holds nothing yet; follow what is appended there with `tap`"
                .to_owned(),
        );
    }
    Ok(if call.input.is(&TypeRef::REPLAYABLE) {
        TypeRef::seq(element)
    } else {
        crate::types::collections::STREAM.apply([element])
    })
}

fn check_read(
    _: &mut dyn CheckCx,
    call: &CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if matches!(io::policy(), Policy::Document { .. }) && call.input.is(&TypeRef::STDIN) {
        return Err(Diagnostic::typing(
            span,
            "a document reads no standard input; read a file beside it",
        ));
    }
    Ok(())
}

fn refuse_in_documents(span: &Range<usize>) -> Result<(), Diagnostic> {
    if matches!(io::policy(), Policy::Document { .. }) {
        return Err(Diagnostic::typing(
            span.clone(),
            "a document writes nothing outside itself, so `target` and `sink` are refused here",
        ));
    }
    Ok(())
}

/// A write to a channel: no format, and values of the channel's type. A
/// channel is the one endpoint a document may write. Answers whether the
/// endpoint is a channel.
fn check_channel_write(
    cx: &mut dyn CheckCx,
    call: &CheckedCall<'_>,
    written: &TypeRef,
    span: &Range<usize>,
) -> Result<bool, Diagnostic> {
    let Some(endpoint) = written_endpoint(call) else {
        return Ok(false);
    };
    let ty = cx.infer(endpoint)?;
    let Some(element) = channel_element(&ty) else {
        refuse_in_documents(span)?;
        return Ok(false);
    };
    if call.bindings.contains_key("F") {
        return Err(Diagnostic::typing(span.clone(), NO_FORMAT));
    }
    if !written.is(element) {
        return Err(Diagnostic::typing(
            span.clone(),
            format!("{ty} holds {element}, not {written}"),
        ));
    }
    Ok(true)
}

fn check_target(
    cx: &mut dyn CheckCx,
    call: &CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if check_channel_write(cx, call, call.input, &span)? {
        return Ok(());
    }
    if call.input.is_stream() && !call.input.is_collection() {
        return Err(Diagnostic::typing(
            span,
            format!(
                "`target` writes one completed value, and {} may not end; write it piece by piece \
                 with `sink`, or bound it with `take(n)` first",
                call.input
            ),
        ));
    }
    let format = checked_format(call, written_endpoint(call), "target")
        .map_err(|message| Diagnostic::typing(span.clone(), message))?;
    crate::extensions::format::writable(call.input, format.0)
        .map_err(|message| Diagnostic::typing(span, message))
}

fn check_sink(
    cx: &mut dyn CheckCx,
    call: &CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    let element = call.input.iterated().unwrap_or(TypeRef::NEVER);
    if check_channel_write(cx, call, &element, &span)? {
        return Ok(());
    }
    let format = checked_format(call, written_endpoint(call), "sink")
        .map_err(|message| Diagnostic::typing(span.clone(), message))?;
    if matches!(format.0, "D2" | "Mermaid") {
        return Err(Diagnostic::typing(
            span,
            format!(
                "{} describes a whole, so it is written by `target`, not `sink`",
                format.0
            ),
        ));
    }
    Ok(())
}

fn url_refused(path: &str) -> String {
    format!("{path:?} is a URL, not a file; a network resource is read with `http`")
}

fn check_file(
    _: &mut dyn CheckCx,
    call: &CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    match call.input_expr.map(|input| &input.kind) {
        Some(Kind::Str(path)) if path.contains("://") => {
            Err(Diagnostic::typing(span, url_refused(path)))
        }
        _ => Ok(()),
    }
}

fn eval_file(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let Value::Str(path) = input else {
        return Err(Diagnostic::runtime(span, "`file` takes a path"));
    };
    if path.contains("://") {
        return Err(Diagnostic::runtime(span, url_refused(&path)));
    }
    Ok(Value::Endpoint(Arc::new(Endpoint::File(PathBuf::from(
        path.as_ref(),
    )))))
}

/// The endpoint a value is.
fn endpoint(value: &Value, span: &Range<usize>) -> Result<Arc<Endpoint>, Diagnostic> {
    match value {
        Value::Endpoint(endpoint) => Ok(Arc::clone(endpoint)),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("{} is not an endpoint", other.type_of()),
        )),
    }
}

/// The format an evaluated call names, or the one its path implies.
fn evaluated_format(
    arguments: &[TypedArg],
    endpoint: &Endpoint,
    span: &Range<usize>,
) -> Result<TypeConstructor, Diagnostic> {
    if let Some(argument) = arguments
        .iter()
        .find(|argument| matches!(argument.value.kind, TypedKind::Type))
    {
        return Ok(argument.value.ty.constructor);
    }
    endpoint
        .path()
        .and_then(io::format_of_extension)
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "no format is named or implied"))
}

fn eval_source(
    _: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let endpoint = endpoint(&input, &span)?;
    if let Endpoint::Channel(name, element) = endpoint.as_ref() {
        return Ok(Value::list(io::held(name), element.clone()));
    }
    let format = evaluated_format(arguments, &endpoint, &span)?;
    let element =
        io::document_type(format).map_err(|message| Diagnostic::runtime(span.clone(), message))?;
    match endpoint.as_ref() {
        Endpoint::File(_) | Endpoint::Http { .. } => {
            let values = io::read_all(&endpoint, format)
                .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
            Ok(Value::Traversal(Arc::new(Traversal::kept(element, values))))
        }
        Endpoint::Stdin | Endpoint::Socket(..) | Endpoint::WebSocket(_) => {
            let reading = Reading::live(&endpoint, format)
                .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
            Ok(live(reading, element, span))
        }
        other => Err(Diagnostic::runtime(
            span,
            format!("{} is not read by `source`", other.describe()),
        )),
    }
}

fn eval_tap(
    _: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let endpoint = endpoint(&input, &span)?;
    if let Endpoint::Channel(name, element) = endpoint.as_ref() {
        let reading = Reading::Channel {
            name: name.clone(),
            from: io::held(name).len(),
        };
        return Ok(live(reading, element.clone(), span));
    }
    let format = evaluated_format(arguments, &endpoint, &span)?;
    let element =
        io::document_type(format).map_err(|message| Diagnostic::runtime(span.clone(), message))?;
    let reading = match endpoint.as_ref() {
        Endpoint::Stdin | Endpoint::Socket(..) | Endpoint::WebSocket(_) => {
            Reading::live(&endpoint, format)
                .map_err(|message| Diagnostic::runtime(span.clone(), message))?
        }
        Endpoint::Http { url, headers, .. } => Reading::Http {
            url: url.clone(),
            headers: headers.clone(),
            format,
        },
        Endpoint::File(path) => Reading::File {
            path: path.clone(),
            format,
        },
        Endpoint::FileEnd(path) => Reading::Follow {
            path: path.clone(),
            format,
        },
        other => {
            return Err(Diagnostic::runtime(
                span,
                format!("{} is not read by `tap`", other.describe()),
            ));
        }
    };
    Ok(live(reading, element, span))
}

fn live(reading: Reading, element: TypeRef, span: Range<usize>) -> Value {
    Value::Traversal(Arc::new(Traversal::new(
        element,
        false,
        Source::Read {
            reading: Arc::new(reading),
            span,
        },
    )))
}

/// The endpoint argument of a write, evaluated.
fn written_to(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    span: &Range<usize>,
) -> Result<Arc<Endpoint>, Diagnostic> {
    let argument = arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some("endpoint"))
        .or_else(|| {
            arguments.iter().find(|argument| {
                argument.name.is_none() && !matches!(argument.value.kind, TypedKind::Type)
            })
        })
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "a write needs an endpoint"))?;
    let value = cx.evaluate(&argument.value)?;
    endpoint(&value, span)
}

fn eval_target(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let endpoint = written_to(cx, arguments, &span)?;
    if let Endpoint::Channel(name, _) = endpoint.as_ref() {
        io::push(name, settled(cx, input)?);
        return Ok(Value::Unit);
    }
    let format = evaluated_format(arguments, &endpoint, &span)?;
    let whole = match &input {
        Value::Traversal(traversal) if traversal.forced().is_none() => {
            let mut values = Vec::new();
            cx.traverse(traversal, &mut |_, value| {
                values.push(value);
                Ok(true)
            })?;
            Value::list(values, traversal.element.clone())
        }
        other => other.clone(),
    };
    let text = crate::extensions::format::render(&whole, format, 0, None, &cx.vault().formats)
        .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
    match endpoint.as_ref() {
        Endpoint::FileEnd(_) => {
            let (mut destination, _) = Destination::open(&endpoint)
                .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
            destination.line(text.trim_end_matches('\n'))
        }
        _ => io::write_whole(&endpoint, text.trim_end_matches('\n')),
    }
    .map_err(|message| Diagnostic::runtime(span, message))?;
    Ok(Value::Unit)
}

fn eval_sink(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let endpoint = written_to(cx, arguments, &span)?;
    if let Endpoint::Channel(name, _) = endpoint.as_ref() {
        match &input {
            Value::Traversal(traversal) => {
                cx.traverse(traversal, &mut |_, value| {
                    io::push(name, value);
                    Ok(true)
                })?;
            }
            other => {
                for value in other.elements().unwrap_or_default() {
                    io::push(name, value);
                }
            }
        }
        return Ok(Value::Unit);
    }
    let format = evaluated_format(arguments, &endpoint, &span)?;
    let (mut destination, held) = Destination::open(&endpoint)
        .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
    // A CSV header is written once: not again when appending to rows.
    let mut first = !held;
    let mut write = |value: Value| -> Result<bool, Diagnostic> {
        let text = crate::extensions::format::piece(&value, format, first)
            .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
        first = false;
        destination
            .line(&text)
            .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
        Ok(true)
    };
    match &input {
        Value::Traversal(traversal) => {
            cx.traverse(traversal, &mut |_, value| write(value))?;
        }
        other => {
            for value in other.elements().unwrap_or_default() {
                write(value)?;
            }
        }
    }
    Ok(Value::Unit)
}

/// A value as it is held: a finite traversal's values produced.
fn settled(cx: &mut dyn EvalCx, value: Value) -> Result<Value, Diagnostic> {
    match &value {
        Value::Traversal(traversal) if traversal.finite && traversal.forced().is_none() => {
            let mut values = Vec::new();
            cx.traverse(traversal, &mut |_, value| {
                values.push(value);
                Ok(true)
            })?;
            Ok(Value::list(values, traversal.element.clone()))
        }
        _ => Ok(value),
    }
}
