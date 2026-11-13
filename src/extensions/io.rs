//! The `io` extension: endpoints read by `source` and `tap`, and written by
//! `target` and `sink`.
//!
//! An endpoint says where values are and a format type how its bytes are
//! documents. Which operation is used says whether the documents are held
//! (`source`, `target`) or taken one at a time (`tap`, `sink`); which endpoint
//! says whether a file is replaced or appended to. The I/O itself is in
//! [`crate::io`]. See [HQL-0021](../../doc/proposals/HQL-0021/README.md).

use super::Signature;
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
use crate::types::{Record, TypeConstructor, TypeRef, Value};
use crate::walk::paths;
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
    variadic: false,
};

const FORMAT_PARAMETER: TypeParameter = TypeParameter {
    name: "F",
    bound: Some(crate::types::builtin::FORMAT),
};

/// A write's endpoint, or the id of a configured endpoint (HQL-0035, HQL-0046).
const TARGET: ArgumentSpec = ArgumentSpec::value(
    "endpoint",
    P::OneOf(&[
        P::Type(TypeConstructor("Writable")),
        P::Type(TypeConstructor("String")),
    ]),
);

/// Every operation the `io` extension provides.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        check: Some(check_file),
        ..super::declared("file", eval_file)
    },
    super::declared("Path", eval_path),
    super::declared("join", eval_join),
    super::declared("parent", eval_parent),
    super::declared("name", eval_name),
    super::declared("stem", eval_stem),
    super::declared("extension", eval_extension),
    Operation {
        name: "source",
        spec: Signature::Native(OperationSpec {
            parameters: &[FORMAT_PARAMETER],
            input: P::OneOf(&[
                P::Type(TypeConstructor("Readable")),
                P::Type(TypeConstructor("String")),
            ]),
            arguments: &[FORMAT],
            output: O::Computed("Seq<T> | Stream<T> | S"),
        }),
        purity: Purity::Effectful,
        shape: super::Shape::Barrier,
        check: Some(check_read),
        eval: eval_source,
    },
    Operation {
        name: "tap",
        spec: Signature::Native(OperationSpec {
            parameters: &[FORMAT_PARAMETER],
            input: P::OneOf(&[
                P::Type(TypeConstructor("Readable")),
                P::Type(TypeConstructor("String")),
            ]),
            arguments: &[FORMAT],
            output: O::Computed("Stream<T>"),
        }),
        purity: Purity::Effectful,
        shape: super::Shape::Barrier,
        check: Some(check_read),
        eval: eval_tap,
    },
    Operation {
        name: "target",
        spec: Signature::Native(OperationSpec {
            parameters: &[
                TypeParameter {
                    name: "T",
                    bound: None,
                },
                FORMAT_PARAMETER,
            ],
            input: P::Var("T"),
            arguments: &[TARGET, FORMAT],
            output: O::Type(P::Type(TypeConstructor("Step"))),
        }),
        purity: Purity::Effectful,
        shape: super::Shape::Barrier,
        check: Some(check_target),
        eval: eval_target,
    },
    Operation {
        name: "sink",
        spec: Signature::Native(OperationSpec {
            parameters: &[
                TypeParameter {
                    name: "T",
                    bound: None,
                },
                FORMAT_PARAMETER,
            ],
            input: P::Iterable(&P::Var("T")),
            arguments: &[TARGET, FORMAT],
            output: O::Type(P::Type(TypeConstructor("Step"))),
        }),
        purity: Purity::Effectful,
        shape: super::Shape::Barrier,
        check: Some(check_sink),
        eval: eval_sink,
    },
    // A configured endpoint as a value, typed from the configuration as
    // `source("id")` is (HQL-0051).
    Operation {
        name: "configured",
        spec: Signature::Native(OperationSpec {
            parameters: &[],
            input: P::Type(TypeConstructor("String")),
            arguments: &[],
            output: O::Computed("Transport"),
        }),
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: None,
        eval: eval_configured,
    },
];

/// The literal id a configured endpoint is named by: `"id"`, or
/// `configured("id")`, which names the same endpoint as a value.
fn literal_id(expression: &Expr) -> Option<&str> {
    match &expression.kind {
        Kind::Str(id) => Some(id),
        Kind::Call { callee, arguments } => match (&callee.kind, arguments.as_slice()) {
            (Kind::Ident(name), [argument]) if name == "configured" && argument.name.is_none() => {
                match &argument.value.kind {
                    Kind::Str(id) => Some(id),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// The id an endpoint expression names for the checker: a literal, a
/// `configured("id")` call, or a binding of one.
fn bound_id(expression: &Expr, cx: &dyn CheckCx) -> Option<String> {
    if let Some(id) = literal_id(expression) {
        return Some(id.to_owned());
    }
    match &expression.kind {
        Kind::Ident(name) => cx.configured_binding(name),
        _ => None,
    }
}

/// The id of a configured endpoint a read or write names, by its literal
/// or by `configured("id")`, which is read and written exactly as the id is.
pub(crate) fn configured_id(value: &Value) -> Option<&str> {
    match value {
        Value::Str(id) => Some(id),
        Value::Endpoint(endpoint) => endpoint.configured_id(),
        _ => None,
    }
}

/// The transport type a configured endpoint's kind states: what
/// `configured("id")` is, read offline. A vault has no transport, and a
/// kind whose transport has no module yet has no type to give.
fn configured_transport(configured: &crate::endpoints::Configured) -> Result<TypeRef, String> {
    use crate::endpoints::Kind as Source;
    let id = &configured.id;
    Ok(match &configured.kind {
        Source::Vault { .. } => {
            return Err(format!(
                "endpoint `{id}` is a vault, which is not a transport; read it with `source`"
            ));
        }
        Source::File { .. } => TypeRef::FILE,
        Source::Sqlite { .. } => TypeRef::SQLITE,
        Source::Postgres { .. } => TypeRef::POSTGRES,
        Source::Spark { .. } => TypeRef::SPARK,
        Source::Nats { .. } => TypeRef::NATS,
        Source::NatsJetStream { .. } => TypeRef::NATS_JET_STREAM,
        kind @ Source::Kafka { .. } => {
            return Err(format!(
                "endpoint `{id}` is a `{}` endpoint, whose transport has no type until its module \
                 is declared (HQL-0048); name it by its id",
                kind.name()
            ));
        }
    })
}

fn eval_configured(
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let Value::Str(id) = &input else {
        return Err(Diagnostic::runtime(
            span,
            format!(
                "`configured` names an endpoint by its id, not {}",
                input.type_of()
            ),
        ));
    };
    let failed = |message: String| Diagnostic::runtime(span.clone(), message);
    let held = configured(id, cx.vault()).map_err(failed)?;
    let ty = configured_transport(held).map_err(failed)?;
    Ok(Value::Endpoint(Arc::new(Endpoint::Configured {
        id: id.to_string(),
        ty,
    })))
}

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
            // Through an age wrapper, the path without `.age` implies the
            // format: `encrypted_to(file("orders.jsonl.age"), …)` is JSON
            // Lines (HQL-0053 part 4).
            (Kind::Ident(name), [inner, ..]) | (Kind::Field(_, name), [inner, ..])
                if matches!(name.as_str(), "encrypted_to" | "decrypted_with") =>
            {
                literal_path(&inner.value).map(|path| path.strip_suffix(".age").unwrap_or(path))
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
    carried: Option<&TypeRef>,
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
    if call.input.is(&TypeRef::STDIN)
        && let Some(format) = io::stdin_format()
    {
        return Ok(format);
    }
    // Each message of a message transport is one document, JSON where the
    // call names no format (HQL-0047).
    if carried.is_some_and(|endpoint| endpoint.is(&TypeRef::MESSAGE_TRANSPORT)) {
        return Ok(TypeConstructor("Json"));
    }
    match endpoint.and_then(literal_path) {
        Some(path) => io::format_of_extension(Path::new(path)).ok_or_else(|| {
            format!("the extension of {path:?} implies no format; name it: `{verb}(…, Json)`")
        }),
        None if call.input.is(&TypeRef::STDIN) => Err(format!(
            "`{verb}` needs a format, `{verb}(stdin, Json)`, or `--from json` on the command line"
        )),
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

const ROWS_TAKE_NO_FORMAT: &str =
    "a table holds rows of named columns, not bytes, so it takes no format";

/// The type `source` or `tap` gives.
pub(crate) fn result(
    name: &str,
    call: &CheckedCall<'_>,
    cx: &dyn CheckCx,
) -> Result<TypeRef, Diagnostic> {
    let vault = cx.vault();
    if name == "configured" {
        let span = call
            .input_expr
            .map_or(0..0, |expression| expression.span.clone());
        let Some(Kind::Str(id)) = call.input_expr.map(|expression| &expression.kind) else {
            return Err(Diagnostic::typing(
                span,
                "`configured` names a configured endpoint by a literal id, \
                 `configured(\"orders\")`: a computed id would leave its type unknown until \
                 evaluation",
            ));
        };
        let held = configured(id, vault).map_err(|message| {
            if crate::endpoints::is_id(id) {
                Diagnostic::name(span.clone(), message)
            } else {
                Diagnostic::typing(span.clone(), message)
            }
        })?;
        return configured_transport(held).map_err(|message| Diagnostic::typing(span, message));
    }
    let bound_id = call
        .input_expr
        .and_then(|expression| bound_id(expression, cx));
    if call.input.is(&TypeRef::STR) || bound_id.is_some() {
        let read = configured_type(call, vault, bound_id.as_deref())?;
        if name != "tap" {
            if let Some(id) = bound_id.as_deref()
                && let Ok(configured) = configured(id, vault)
                && configured.kind.is_live()
            {
                return Err(Diagnostic::typing(
                    call.input_expr
                        .map_or(0..0, |expression| expression.span.clone()),
                    live_by_source(id, &configured.kind),
                ));
            }
            return Ok(read);
        }
        // `tap("id")` holds one element at a time: a finite source's
        // elements as a Stream.
        return match read.iterated() {
            Some(element) if read != TypeRef::VAULT => {
                Ok(crate::types::collections::STREAM.apply([element]))
            }
            _ => Err(Diagnostic::typing(
                call.input_expr
                    .map_or(0..0, |expression| expression.span.clone()),
                "a vault source holds cards as a field, not as its elements; read it with \
                 `source`, and its cards with `.cards`",
            )),
        };
    }
    let span = call
        .input_expr
        .map_or(0..0, |expression| expression.span.clone());
    read_type(name, call).map_err(|message| Diagnostic::typing(span, message))
}

/// The type of `source("id")`: the one the run context's configuration
/// states for the id, read offline. A vault source is a `Vault`, and a
/// file or table a `Seq` of the element its configuration names, or of the
/// kind's own document type.
fn configured_type(
    call: &CheckedCall<'_>,
    vault: &crate::vault::Vault,
    bound: Option<&str>,
) -> Result<TypeRef, Diagnostic> {
    let span = call
        .input_expr
        .map_or(0..0, |expression| expression.span.clone());
    let Some(id) = bound else {
        return Err(Diagnostic::typing(
            span,
            "`source` names a configured endpoint by a literal id, `source(\"field_notes\")`: \
             a computed id would leave its type unknown until evaluation",
        ));
    };
    if call.bindings.contains_key("F") {
        return Err(Diagnostic::typing(
            span,
            "a configured endpoint's format is its kind's, set in its configuration; name none",
        ));
    }
    let configured = configured(id, vault).map_err(|message| {
        if crate::endpoints::is_id(id) {
            Diagnostic::name(span.clone(), message)
        } else {
            Diagnostic::typing(span.clone(), message)
        }
    })?;
    if matches!(configured.kind, crate::endpoints::Kind::Vault { .. }) {
        return Ok(TypeRef::VAULT);
    }
    let element = match &configured.element {
        Some(written) => crate::types::named(written, &[]).ok_or_else(|| {
            Diagnostic::typing(
                span.clone(),
                format!("endpoint `{id}` names the element type `{written}`, which is not a type"),
            )
        })?,
        None => TypeRef::DATA,
    };
    // What the boundary says of the element type is checked here; the
    // external fields are known only when the source is read.
    boundary_for(configured, &element, None)
        .map_err(|message| Diagnostic::typing(span.clone(), message))?;
    Ok(TypeRef::seq(element))
}

/// Why `source` does not read a topic or a stream's subject: a log never
/// ends.
fn live_by_source(id: &str, kind: &crate::endpoints::Kind) -> String {
    format!(
        "`{id}` is a `{}` endpoint, read by `tap` as a `Stream`, not by `source` as a `Seq`",
        kind.name()
    )
}

/// The configuration of the endpoint an id names, for a statement over it
/// (HQL-0051).
pub(crate) fn configured_endpoint<'v>(
    id: &str,
    vault: &'v crate::vault::Vault,
) -> Result<&'v crate::endpoints::Configured, String> {
    configured(id, vault)
}

/// The literal id an endpoint expression names, `"id"` or
/// `configured("id")`, for the checker.
pub(crate) fn literal_endpoint_id(expression: &Expr) -> Option<&str> {
    literal_id(expression)
}

/// A configured endpoint's kind and the system it is in, for a
/// transaction's participants: the database file of a `sqlite` endpoint,
/// the variable naming a `postgres` endpoint's connection string.
pub(crate) fn configured_system(
    id: &str,
    vault: &crate::vault::Vault,
) -> Result<(String, Option<String>), String> {
    use crate::endpoints::Kind as Source;
    let held = configured(id, vault)?;
    let system = match &held.kind {
        Source::Sqlite { path, .. } => Some(path.display().to_string()),
        Source::Postgres { dsn_env, .. } => Some(dsn_env.clone()),
        Source::Spark { server, .. } => Some(match server {
            crate::endpoints::Address::Written(url) | crate::endpoints::Address::Env(url) => {
                url.clone()
            }
        }),
        _ => None,
    };
    Ok((held.kind.name().to_owned(), system))
}

/// The element type a configured endpoint names, `element = "Order"`:
/// `None` where it names none.
pub(crate) fn configured_element(
    id: &str,
    vault: &crate::vault::Vault,
) -> Result<Option<TypeRef>, String> {
    let held = configured(id, vault)?;
    match &held.element {
        Some(written) => crate::types::named(written, &[]).map(Some).ok_or_else(|| {
            format!("endpoint `{id}` names the element type `{written}`, which is not a type")
        }),
        None => Ok(None),
    }
}

/// The configuration of the source an id names: the id must be one, and
/// configured.
fn configured<'v>(
    id: &str,
    vault: &'v crate::vault::Vault,
) -> Result<&'v crate::endpoints::Configured, String> {
    if !crate::endpoints::is_id(id) {
        return Err(
            if id.contains(':') || id.contains('/') || id.contains('.') {
                format!(
                    "{id:?} is not an endpoint id: connection details belong in configuration, \
                 `[endpoint.<id>]` in hql.toml, and a program names the id"
                )
            } else {
                format!("{id:?} is not an endpoint id: an id is snake_case, `[a-z][a-z0-9_]*`")
            },
        );
    }
    vault.endpoints.get(id).ok_or_else(|| {
        let hint = vault.endpoints.nearest(id).map_or_else(
            || {
                "; an endpoint is declared under `[endpoint.<id>]` in the vault's hql.toml"
                    .to_owned()
            },
            |near| format!("; did you mean `{near}`?"),
        );
        format!("no endpoint `{id}` is configured{hint}")
    })
}

/// The type a read of an endpoint gives.
fn read_type(name: &str, call: &CheckedCall<'_>) -> Result<TypeRef, String> {
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
    // A row has columns, and no bytes to decode (HQL-0047).
    if call.input.is(&TypeRef::ROW_TRANSPORT) {
        if call.bindings.contains_key("F") {
            return Err(ROWS_TAKE_NO_FORMAT.to_owned());
        }
        return Ok(if name == "tap" {
            crate::types::collections::STREAM.apply([TypeRef::DATA])
        } else {
            TypeRef::seq(TypeRef::DATA)
        });
    }
    let format = checked_format(call, call.input_expr, Some(call.input), name)?;
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
    if io::program_on_stdin() && call.input.is(&TypeRef::STDIN) {
        return Err(Diagnostic::typing(
            span,
            "this program was read from standard input, so standard input holds no data; run it from a file",
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
    if let Some(id) = written_endpoint(call).and_then(|endpoint| bound_id(endpoint, cx)) {
        return check_configured_target(cx.vault(), &id, call, &span);
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
    let carried = written_endpoint(call).and_then(|endpoint| cx.infer(endpoint).ok());
    if carried
        .as_ref()
        .is_some_and(|endpoint| endpoint.is(&TypeRef::ROW_TRANSPORT))
    {
        if call.bindings.contains_key("F") {
            return Err(Diagnostic::typing(span, ROWS_TAKE_NO_FORMAT));
        }
        let element = call
            .input
            .iterated()
            .or_else(|| call.input.element())
            .unwrap_or(TypeRef::NEVER);
        if declared_fields(&element).is_none() {
            return Err(Diagnostic::typing(
                span,
                format!("a table is written rows of a declared record, and {element} is not one"),
            ));
        }
        return Ok(());
    }
    let format = checked_format(call, written_endpoint(call), carried.as_ref(), "target")
        .map_err(|message| Diagnostic::typing(span.clone(), message))?;
    crate::extensions::format::writable(call.input, format.0)
        .map_err(|message| Diagnostic::typing(span, message))
}

/// `xs | target("id")`: the id names a configured endpoint that is not a
/// vault, no format is named beside it, and what is written is a table or a
/// sequence of a declared record, which the boundary accepts.
fn check_configured_target(
    vault: &crate::vault::Vault,
    id: &str,
    call: &CheckedCall<'_>,
    span: &Range<usize>,
) -> Result<(), Diagnostic> {
    let configured = configured(id, vault).map_err(|message| {
        if crate::endpoints::is_id(id) {
            Diagnostic::name(span.clone(), message)
        } else {
            Diagnostic::typing(span.clone(), message)
        }
    })?;
    if matches!(configured.kind, crate::endpoints::Kind::Vault { .. }) {
        return Err(Diagnostic::typing(
            span.clone(),
            format!("endpoint `{id}` is a vault, which is read and never written"),
        ));
    }
    if call.bindings.contains_key("F") {
        return Err(Diagnostic::typing(
            span.clone(),
            "a configured endpoint's format is its kind's, set in its configuration; name none",
        ));
    }
    let element = if call.input.constructor.0 == "Table" {
        call.input.args.first().cloned()
    } else {
        call.input.element().filter(|_| call.input.is_collection())
    }
    .ok_or_else(|| {
        Diagnostic::typing(
            span.clone(),
            format!(
                "`target(\"{id}\")` writes a Table or a sequence of a declared record, not {}",
                call.input
            ),
        )
    })?;
    match boundary_for(configured, &element, None) {
        Ok(Some(_)) => Ok(()),
        Ok(None) if configured.kind.is_live() => Ok(()),
        Ok(None) => Err(Diagnostic::typing(
            span.clone(),
            format!(
                "`target(\"{id}\")` writes rows of a declared record, and {element} is not one"
            ),
        )),
        Err(message) => Err(Diagnostic::typing(span.clone(), message)),
    }
}

/// `xs | sink("id")`: the id names a Kafka topic, which takes any value as
/// a message, or a PostgreSQL table, which takes rows of a declared record.
fn check_configured_sink(
    vault: &crate::vault::Vault,
    id: &str,
    call: &CheckedCall<'_>,
    element: &TypeRef,
    span: &Range<usize>,
) -> Result<(), Diagnostic> {
    use crate::endpoints::Kind as Source;
    refuse_in_documents(span)?;
    let configured = configured(id, vault).map_err(|message| {
        if crate::endpoints::is_id(id) {
            Diagnostic::name(span.clone(), message)
        } else {
            Diagnostic::typing(span.clone(), message)
        }
    })?;
    if call.bindings.contains_key("F") {
        return Err(Diagnostic::typing(
            span.clone(),
            "a configured endpoint's format is its kind's, set in its configuration; name none",
        ));
    }
    let boundary = boundary_for(configured, element, None)
        .map_err(|message| Diagnostic::typing(span.clone(), message))?;
    match &configured.kind {
        Source::Kafka { .. } | Source::Nats { .. } | Source::NatsJetStream { .. } => Ok(()),
        Source::Postgres { .. } | Source::Spark { .. } if boundary.is_some() => Ok(()),
        Source::Postgres { .. } | Source::Spark { .. } => Err(Diagnostic::typing(
            span.clone(),
            format!("`sink(\"{id}\")` writes rows of a declared record, and {element} is not one"),
        )),
        other => Err(Diagnostic::typing(
            span.clone(),
            format!(
                "endpoint `{id}` is a `{}` endpoint, which `sink` does not write; write it whole \
                 with `target`",
                other.name()
            ),
        )),
    }
}

fn check_sink(
    cx: &mut dyn CheckCx,
    call: &CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    let element = call.input.iterated().unwrap_or(TypeRef::NEVER);
    if let Some(id) = written_endpoint(call).and_then(|endpoint| bound_id(endpoint, cx)) {
        return check_configured_sink(cx.vault(), &id, call, &element, &span);
    }
    if check_channel_write(cx, call, &element, &span)? {
        return Ok(());
    }
    let carried = written_endpoint(call).and_then(|endpoint| cx.infer(endpoint).ok());
    if carried
        .as_ref()
        .is_some_and(|endpoint| endpoint.is(&TypeRef::ROW_TRANSPORT))
    {
        return Err(Diagnostic::typing(
            span,
            "a table of a database file is written whole by `target`, not row by row by `sink`",
        ));
    }
    let format = checked_format(call, written_endpoint(call), carried.as_ref(), "sink")
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
    let path = match input {
        Value::Str(path) => path,
        record @ Value::Record(_) => Arc::from(path_text(&record, "file", &span)?),
        _ => return Err(Diagnostic::runtime(span, "`file` takes a path")),
    };
    if path.contains("://") {
        return Err(Diagnostic::runtime(span, url_refused(&path)));
    }
    Ok(Value::Endpoint(Arc::new(Endpoint::File(PathBuf::from(
        path.as_ref(),
    )))))
}

/// The `Path` value whose text is `text`.
pub(crate) fn path_value(text: &str) -> Value {
    Value::Record(Arc::new(Record {
        ty: TypeRef::PATH,
        fields: vec![("text".to_owned(), Value::Str(Arc::from(text)))],
        carrier: None,
    }))
}

/// The text of a `Path` value.
pub(crate) fn path_text(
    value: &Value,
    name: &str,
    span: &Range<usize>,
) -> Result<String, Diagnostic> {
    match value {
        Value::Present(held) => path_text(held, name, span),
        Value::Record(record) if record.ty == TypeRef::PATH => match record.field("text") {
            Some(Value::Str(text)) => Ok(text.to_string()),
            _ => Err(Diagnostic::runtime(
                span.clone(),
                format!("`{name}` takes a Path"),
            )),
        },
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` takes a Path, not {}", other.type_of()),
        )),
    }
}

fn eval_path(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    match input {
        Value::Str(text) => Ok(path_value(&text)),
        other => Err(Diagnostic::runtime(
            span,
            format!("`Path` takes a String, not {}", other.type_of()),
        )),
    }
}

fn eval_join(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let text = path_text(&input, "join", &span)?;
    let segment = match super::named(arguments, "segment").or_else(|| arguments.first()) {
        Some(argument) => cx.evaluate(&argument.value)?,
        None => return Err(Diagnostic::runtime(span, "`join` takes a segment")),
    };
    let Value::Str(segment) = segment else {
        return Err(Diagnostic::runtime(
            span,
            format!(
                "the segment of `join` is a String, not {}",
                segment.type_of()
            ),
        ));
    };
    Ok(path_value(&paths::join(&text, &segment)))
}

fn eval_parent(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let text = path_text(&input, "parent", &span)?;
    Ok(match paths::parent(&text) {
        Some(parent) => Value::Present(Arc::new(path_value(parent))),
        None => Value::Absent(TypeRef::PATH),
    })
}

fn eval_name(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let text = path_text(&input, "name", &span)?;
    Ok(Value::Str(Arc::from(paths::name(&text))))
}

fn eval_stem(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let text = path_text(&input, "stem", &span)?;
    Ok(Value::Str(Arc::from(paths::stem(&text))))
}

fn eval_extension(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let text = path_text(&input, "extension", &span)?;
    Ok(match paths::extension(&text) {
        Some(extension) => Value::Present(Arc::new(Value::Str(Arc::from(extension)))),
        None => Value::Absent(TypeRef::STR),
    })
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
    if matches!(endpoint, Endpoint::Stdin)
        && let Some(format) = io::stdin_format()
    {
        return Ok(format);
    }
    if endpoint.type_of().is(&TypeRef::MESSAGE_TRANSPORT) {
        return Ok(TypeConstructor("Json"));
    }
    endpoint
        .format_path()
        .and_then(|path| io::format_of_extension(&path))
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "no format is named or implied"))
}

/// The rows of a table transport, each a `Data` map from column to value;
/// `None` for an endpoint that is not one.
fn table_rows(endpoint: &Endpoint, span: &Range<usize>) -> Result<Option<Vec<Value>>, Diagnostic> {
    let failed = |message: String| Diagnostic::io_failure(span.clone(), message);
    let located = |message: String| failed(format!("{}: {message}", endpoint.describe()));
    let rows = match endpoint {
        Endpoint::Sqlite { path, table, order } => {
            let full = io::readable_database(path).map_err(failed)?;
            crate::endpoints::sqlite_rows(&full, table, order, None).map_err(located)?
        }
        Endpoint::Postgres {
            dsn_env,
            table,
            order,
        } => {
            io::policy_permits_network().map_err(located)?;
            let dsn = crate::endpoints::secret(dsn_env, "dsn_env").map_err(located)?;
            crate::postgres::rows(&dsn, table, order, None).map_err(located)?
        }
        Endpoint::Spark {
            server,
            token_env,
            table,
            order,
        } => {
            io::policy_permits_network().map_err(located)?;
            let constructed =
                constructed_table(endpoint).ok_or_else(|| located("not a table".to_owned()))?;
            let server =
                spark_server(&constructed, server, token_env.as_deref()).map_err(located)?;
            crate::spark::rows(&server, table, order, None).map_err(located)?
        }
        _ => return Ok(None),
    };
    Ok(Some(
        rows.into_iter()
            .map(|row| Value::Data(Arc::new(row)))
            .collect(),
    ))
}

/// A constructed table as a configured endpoint with no boundary of its
/// own, so that it reads and writes as its kind does; `None` for an
/// endpoint that is not one.
pub(crate) fn constructed_table(endpoint: &Endpoint) -> Option<crate::endpoints::Configured> {
    let kind = match endpoint {
        Endpoint::Sqlite { path, table, order } => crate::endpoints::Kind::Sqlite {
            path: io::database_path(path),
            table: table.clone(),
            order: order.clone(),
        },
        Endpoint::Postgres {
            dsn_env,
            table,
            order,
        } => crate::endpoints::Kind::Postgres {
            dsn_env: dsn_env.clone(),
            table: table.clone(),
            order: order.clone(),
        },
        Endpoint::Spark {
            server,
            token_env,
            table,
            order,
        } => crate::endpoints::Kind::Spark {
            server: server.clone(),
            token_env: token_env.clone(),
            table: table.clone(),
            order: order.clone(),
        },
        _ => return None,
    };
    Some(crate::endpoints::Configured::constructed(
        endpoint.to_string(),
        kind,
    ))
}

fn eval_source(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Some(id) = configured_id(&input) {
        return read_configured(id, cx.vault(), None, &span);
    }
    let endpoint = endpoint(&input, &span)?;
    if let Endpoint::Channel(name, element) = endpoint.as_ref() {
        return Ok(Value::list(io::held(name), element.clone()));
    }
    if let Some(rows) = table_rows(&endpoint, &span)? {
        return Ok(Value::Traversal(Arc::new(Traversal::kept(
            TypeRef::DATA,
            rows,
        ))));
    }
    let format = evaluated_format(arguments, &endpoint, &span)?;
    let element = io::document_type(format)
        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
    match endpoint.as_ref() {
        Endpoint::Decrypted { .. } if !endpoint.type_of().is(&TypeRef::REPLAYABLE) => {
            Ok(live(Reading::decrypted(&endpoint, format), element, span))
        }
        Endpoint::File(_) | Endpoint::Http { .. } | Endpoint::Decrypted { .. } => {
            let values = io::read_all(&endpoint, format)
                .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
            Ok(Value::Traversal(Arc::new(Traversal::kept(element, values))))
        }
        Endpoint::Stdin
        | Endpoint::Socket(..)
        | Endpoint::WebSocket(_)
        | Endpoint::Nats { .. }
        | Endpoint::NatsJetStream { .. } => {
            let reading = Reading::live(&endpoint, format)
                .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
            Ok(live(reading, element, span))
        }
        other => Err(Diagnostic::runtime(
            span,
            format!("{} is not read by `source`", other.describe()),
        )),
    }
}

fn eval_tap(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Some(id) = configured_id(&input) {
        return tap_configured(id, cx.vault(), &span);
    }
    let endpoint = endpoint(&input, &span)?;
    if let Endpoint::Channel(name, element) = endpoint.as_ref() {
        let reading = Reading::Channel {
            name: name.clone(),
            from: io::held(name).len(),
        };
        return Ok(live(reading, element.clone(), span));
    }
    if let Some(rows) = table_rows(&endpoint, &span)? {
        let streamed = Traversal::new(TypeRef::DATA, false, crate::traversal::Source::Kept);
        streamed.keep(rows);
        return Ok(Value::Traversal(Arc::new(streamed)));
    }
    let format = evaluated_format(arguments, &endpoint, &span)?;
    let element = io::document_type(format)
        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
    let reading = match endpoint.as_ref() {
        Endpoint::Stdin
        | Endpoint::Socket(..)
        | Endpoint::WebSocket(_)
        | Endpoint::Nats { .. }
        | Endpoint::NatsJetStream { .. } => Reading::live(&endpoint, format)
            .map_err(|message| Diagnostic::io_failure(span.clone(), message))?,
        Endpoint::Http {
            url,
            headers,
            timeout,
            ..
        } => Reading::Http {
            url: url.clone(),
            headers: headers.clone(),
            timeout: *timeout,
            format,
        },
        Endpoint::File(path) => Reading::file(path.clone(), format),
        Endpoint::Decrypted { .. } => Reading::decrypted(&endpoint, format),
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

/// The fields of a declared record a boundary maps, each optional one at
/// its present type; `None` for a type that is not a record, which is read
/// as it is.
fn declared_fields(element: &TypeRef) -> Option<Vec<crate::boundary::DeclaredField>> {
    if !crate::types::program::is_record_valued(element.constructor.0) {
        return None;
    }
    let shape = crate::types::program::record(element.constructor.0)?;
    Some(
        shape
            .fields
            .iter()
            .map(|field| crate::boundary::DeclaredField {
                name: field.name.clone(),
                ty: shape.bound(&field.ty, &element.args),
                optional: field.optional,
            })
            .collect(),
    )
}

/// The effective boundary of a configured endpoint read as `element`, checked
/// against `external` where the format knows its fields; `None` where the
/// element is not a record.
///
/// # Errors
/// Every boundary error, one per line.
pub(crate) fn boundary_for(
    configured: &crate::endpoints::Configured,
    element: &TypeRef,
    external: Option<&[String]>,
) -> Result<Option<crate::boundary::EffectiveBoundary>, String> {
    let declared = match &configured.kind {
        crate::endpoints::Kind::Sqlite { path, table, .. } if external.is_some() => {
            Some(crate::endpoints::sqlite_column_types(path, table)?)
        }
        crate::endpoints::Kind::Postgres { dsn_env, table, .. } if external.is_some() => Some(
            crate::postgres::columns(&dsn(configured, dsn_env)?, table)
                .map_err(|message| format!("endpoint `{}`: {message}", configured.id))?
                .into_iter()
                .collect(),
        ),
        crate::endpoints::Kind::Spark {
            server,
            token_env,
            table,
            ..
        } if external.is_some() => Some(
            crate::spark::columns(
                &spark_server(configured, server, token_env.as_deref())?,
                table,
            )
            .map_err(|message| format!("endpoint `{}`: {message}", configured.id))?
            .into_iter()
            .collect(),
        ),
        _ => None,
    };
    let Some(fields) = declared_fields(element) else {
        return Ok(None);
    };
    let none = std::collections::BTreeMap::new();
    crate::boundary::resolve(
        &configured.id,
        configured.kind.name(),
        &fields,
        crate::boundary::Layers {
            configuration: &configured.fields,
            invocation: &none,
            declared: declared.as_ref(),
        },
        external,
    )
    .map(|mut boundary| {
        boundary.strictness = configured.strictness;
        Some(boundary)
    })
    .map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// A document's unmapped external fields under a `warn` boundary, each
/// reported once per read; under `strict`, the boundary error.
///
/// # Errors
/// Names a strict source's unmapped fields.
pub(crate) fn strictness_of(
    boundary: &crate::boundary::EffectiveBoundary,
    document: &Value,
    reported: &mut Vec<String>,
    warn: &mut dyn FnMut(String),
) -> Result<(), String> {
    use crate::endpoints::Strictness;
    let extra = unmapped(document, boundary);
    match boundary.strictness {
        Strictness::Strict if !extra.is_empty() => Err(format!(
            "endpoint `{}` is strict, and its boundary reads no field {}",
            boundary.source,
            extra
                .iter()
                .map(|key| format!("`{key}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Strictness::Warn => {
            for key in extra {
                if !reported.contains(&key) {
                    warn(format!(
                        "endpoint `{}`: its boundary reads no field `{key}`, which is not read",
                        boundary.source
                    ));
                    reported.push(key);
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The external fields of a document no readable HQL field reads, by its
/// name or as the first segment of its path.
pub(crate) fn unmapped(
    document: &Value,
    boundary: &crate::boundary::EffectiveBoundary,
) -> Vec<String> {
    use crate::data::Data;
    let Value::Data(tree) = document else {
        return Vec::new();
    };
    let Data::Map(entries) = tree.as_ref() else {
        return Vec::new();
    };
    entries
        .keys()
        .filter(|key| !boundary_reads(boundary, key))
        .cloned()
        .collect()
}

/// Whether a readable field of the boundary reads the external field `key`.
fn boundary_reads(boundary: &crate::boundary::EffectiveBoundary, key: &str) -> bool {
    use crate::endpoints::External;
    boundary.fields.iter().any(|field| {
        field.readable.value
            && match &field.external.value {
                External::Name(name) => crate::boundary::same_name(&boundary.kind, name, key),
                External::Path(segments) => segments.first().is_some_and(|first| first == key),
            }
    })
}

/// A document's entries as the boundary names them (HQL-0035): each
/// readable field's external entry under the field's own name, an entry its
/// `absent` lists left out so the field is absent or missing, and a field
/// the boundary does not read left out, so its default or absence applies.
/// Entries the boundary does not name are kept.
///
/// # Errors
/// Names an optional field holding a `null` its `absent` does not list.
pub(crate) fn remapped(
    document: &Value,
    boundary: &crate::boundary::EffectiveBoundary,
) -> Result<Value, String> {
    use crate::data::Data;
    use crate::endpoints::{Absence, External};
    let Value::Data(tree) = document else {
        return Ok(document.clone());
    };
    let Data::Map(entries) = tree.as_ref() else {
        return Ok(document.clone());
    };
    use crate::endpoints::Strictness;
    let mut mapped = entries.clone();
    let sqlite = boundary.kind == "sqlite";
    // A strict source's unmapped fields were refused before; an open one's
    // are not read.
    if matches!(boundary.strictness, Strictness::Open | Strictness::Warn) {
        for key in &unmapped(document, boundary) {
            mapped.remove(key);
        }
    }
    // Every entry a field reads, and every field's own name, is cleared
    // before any is filled, so two fields that swap names keep both values,
    // and a path's top-level container is not left behind as a field.
    for field in &boundary.fields {
        match &field.external.value {
            External::Name(external) => mapped.remove(external),
            External::Path(segments) => segments
                .first()
                .and_then(|container| mapped.remove(container)),
        };
        mapped.remove(&field.field.name);
    }
    for field in &boundary.fields {
        let name = &field.field.name;
        if !field.readable.value {
            continue;
        }
        let held = match &field.external.value {
            External::Name(external) => entries.get(external),
            External::Path(segments) => {
                segments
                    .iter()
                    .try_fold(tree.as_ref(), |at, segment| match at {
                        Data::Map(inner) => inner.get(segment),
                        _ => None,
                    })
            }
        };
        let representation = match held {
            None => Some(Absence::Missing),
            Some(Data::Empty) => Some(Absence::Null),
            Some(Data::Str(text)) if text.is_empty() => Some(Absence::Empty),
            Some(_) => None,
        };
        let absent = representation.is_some_and(|held| field.absent.value.contains(&held));
        if absent {
            continue;
        }
        match (held, representation) {
            (Some(Data::Empty), _) if field.field.optional => {
                return Err(format!(
                    "endpoint `{}`, field `{name}`: it holds null, and its boundary reads only {} \
                     as absent",
                    boundary.source,
                    field
                        .absent
                        .value
                        .iter()
                        .map(|absence| format!("`{}`", absence.name()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            // SQLite has no boolean storage: a `Bool` is the `INTEGER` 0 or 1.
            (Some(Data::Int(n)), _) if sqlite && field.field.ty == TypeRef::BOOL => {
                let truth = match n {
                    0 => false,
                    1 => true,
                    _ => {
                        return Err(format!(
                            "endpoint `{}`, field `{name}`: {n} is not a Bool, which a table \
                             holds as 0 or 1",
                            boundary.source
                        ));
                    }
                };
                mapped.insert(name.clone(), Data::Bool(truth));
            }
            // An integer is a number the external type decodes to `Float`,
            // where the Float holds it exactly.
            (Some(Data::Int(n)), _) if field.field.ty == TypeRef::FLOAT => {
                #[allow(clippy::cast_precision_loss)]
                let real = *n as f64;
                if n.unsigned_abs() > 1 << f64::MANTISSA_DIGITS {
                    return Err(format!(
                        "endpoint `{}`, field `{name}`: {n} has more digits than a Float holds",
                        boundary.source
                    ));
                }
                mapped.insert(name.clone(), Data::Float(real));
            }
            // A table's REAL storage may have lost a decimal's digits before
            // it was read, so a `Decimal` takes it only where it may.
            (Some(Data::Float(real)), _)
                if sqlite && field.field.ty == TypeRef::DECIMAL && !field.lossy.value =>
            {
                return Err(format!(
                    "endpoint `{}`, field `{name}`: lossy representation: the column holds the \
                     REAL {real}, which may not keep a Decimal's digits; store it as TEXT, or \
                     set `lossy = true`",
                    boundary.source
                ));
            }
            // A `Decimal` written as text reads the text's digits.
            (Some(Data::Str(text)), _) if field.field.ty == TypeRef::DECIMAL => {
                let digits = crate::decimal::Decimal::parse(text).ok_or_else(|| {
                    format!(
                        "endpoint `{}`, field `{name}`: {text:?} is not a decimal number",
                        boundary.source
                    )
                })?;
                mapped.insert(name.clone(), Data::Decimal(digits));
            }
            (Some(value), _) => {
                mapped.insert(name.clone(), value.clone());
            }
            (None, _) => {}
        }
    }
    Ok(Value::Data(Arc::new(Data::Map(mapped))))
}

/// `source("id")` expected to give a `Seq` of a declared record: the source
/// read through its boundary (HQL-0035). A table reads only the columns the
/// boundary reads; a table's columns and a CSV file's header are checked
/// against the boundary before any element is decoded.
pub(crate) fn configured_source(
    vault: &crate::vault::Vault,
    id: &str,
    wanted: &TypeRef,
    span: &Range<usize>,
    warn: &mut dyn FnMut(String),
) -> Result<Value, Diagnostic> {
    use crate::endpoints::Kind as Source;
    let failed = |message: String| Diagnostic::io_failure(span.clone(), message);
    let configured = configured(id, vault).map_err(failed)?;
    let element = wanted.args.first().cloned().unwrap_or(TypeRef::DATA);
    let endpoint = Value::Str(id.into());
    let plain = |columns: Option<&[String]>| read_configured(id, vault, columns, span);
    let boundary = match &configured.kind {
        Source::Vault { .. } => None,
        Source::Sqlite { path, table, .. } => {
            let columns = crate::endpoints::sqlite_columns(path, table)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            let boundary = boundary_for(configured, &element, Some(&columns)).map_err(failed)?;
            // A table's unmapped columns are never selected, so its
            // strictness is applied to them here, before any row.
            if let Some(boundary) = &boundary {
                let row: std::collections::BTreeMap<String, crate::data::Data> = columns
                    .iter()
                    .map(|column| (column.clone(), crate::data::Data::Empty))
                    .collect();
                let header = Value::Data(Arc::new(crate::data::Data::Map(row)));
                strictness_of(boundary, &header, &mut Vec::new(), warn).map_err(failed)?;
            }
            boundary
        }
        Source::Postgres { dsn_env, table, .. } => {
            let dsn = dsn(configured, dsn_env).map_err(failed)?;
            let columns: Vec<String> = crate::postgres::columns(&dsn, table)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            let boundary = boundary_for(configured, &element, Some(&columns)).map_err(failed)?;
            if let Some(boundary) = &boundary {
                let row: std::collections::BTreeMap<String, crate::data::Data> = columns
                    .iter()
                    .map(|column| (column.clone(), crate::data::Data::Empty))
                    .collect();
                let header = Value::Data(Arc::new(crate::data::Data::Map(row)));
                strictness_of(boundary, &header, &mut Vec::new(), warn).map_err(failed)?;
            }
            boundary
        }
        Source::Spark {
            server,
            token_env,
            table,
            ..
        } => {
            let server = spark_server(configured, server, token_env.as_deref()).map_err(failed)?;
            let columns: Vec<String> = crate::spark::columns(&server, table)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            let boundary = boundary_for(configured, &element, Some(&columns)).map_err(failed)?;
            if let Some(boundary) = &boundary {
                let row: std::collections::BTreeMap<String, crate::data::Data> = columns
                    .iter()
                    .map(|column| (column.clone(), crate::data::Data::Empty))
                    .collect();
                let header = Value::Data(Arc::new(crate::data::Data::Map(row)));
                strictness_of(boundary, &header, &mut Vec::new(), warn).map_err(failed)?;
            }
            boundary
        }
        kind @ (Source::Kafka { .. } | Source::Nats { .. } | Source::NatsJetStream { .. }) => {
            return Err(failed(live_by_source(id, kind)));
        }
        Source::File { .. } => boundary_for(configured, &element, None).map_err(failed)?,
    };
    let Some(boundary) = boundary else {
        let produced = plain(None)?;
        return decoded(vault, &endpoint, produced, wanted, &[], span);
    };
    let read: Vec<String> = boundary
        .fields
        .iter()
        .filter(|field| field.readable.value)
        .filter_map(|field| match &field.external.value {
            crate::endpoints::External::Name(name) => Some(name.clone()),
            crate::endpoints::External::Path(_) => None,
        })
        .collect();
    let columns = matches!(
        configured.kind,
        Source::Sqlite { .. } | Source::Postgres { .. } | Source::Spark { .. }
    )
    .then_some(read.as_slice());
    let Value::Traversal(produced) = plain(columns)? else {
        return Err(failed(format!(
            "endpoint `{id}` is not a sequence of elements"
        )));
    };
    let documents = produced.forced().map(<[Value]>::to_vec).unwrap_or_default();
    // A CSV file's header is the keys of its rows.
    if matches!(&configured.kind, Source::File { format, .. } if *format == "Csv")
        && let Some(Value::Data(first)) = documents.first()
        && let crate::data::Data::Map(entries) = first.as_ref()
    {
        let header: Vec<String> = entries.keys().cloned().collect();
        boundary_for(configured, &element, Some(&header)).map_err(failed)?;
    }
    let leaves = if matches!(&configured.kind, Source::File { format, .. } if *format == "Csv") {
        crate::types::Leaves::Text
    } else {
        crate::types::Leaves::AsWritten
    };
    let from = format!("endpoint `{id}`");
    let mut reported = Vec::new();
    for document in &documents {
        strictness_of(&boundary, document, &mut reported, warn).map_err(failed)?;
    }
    let decoded = documents
        .iter()
        .enumerate()
        .map(|(index, document)| {
            decode_document(
                &remapped(document, &boundary)?,
                &element,
                leaves,
                index + 1,
                &from,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(failed)?;
    Ok(Value::Traversal(Arc::new(Traversal::kept(
        element, decoded,
    ))))
}

/// A configured endpoint's contents: a vault endpoint's vault, or every element
/// of a file or table, in the source's order, within the run context's
/// bounds; of a table, only `columns` where a boundary names them.
/// Configuration is the access boundary, so a document reads every source
/// its vault configures.
fn read_configured(
    id: &str,
    vault: &crate::vault::Vault,
    columns: Option<&[String]>,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    use crate::endpoints::Kind as Source;
    let failed = |message: String| Diagnostic::io_failure(span.clone(), message);
    let configured = configured(id, vault).map_err(failed)?;
    let bounds = vault.endpoints.bounds;
    let within = |count: usize| match bounds.max_elements {
        Some(most) if count > most => Err(failed(format!(
            "endpoint `{id}` holds {count} elements, more than `max_elements = {most}`; take it \
             in portions"
        ))),
        _ => Ok(()),
    };
    let sized = |path: &Path| match bounds.max_bytes {
        Some(most) => match std::fs::metadata(path) {
            Ok(metadata) if metadata.len() > most => Err(failed(format!(
                "endpoint `{id}` is {} bytes, more than `max_bytes = {most}`",
                metadata.len()
            ))),
            _ => Ok(()),
        },
        None => Ok(()),
    };
    let documents = match &configured.kind {
        Source::Vault { .. } => {
            let loaded = configured.vault().map_err(failed)?;
            within(loaded.cards.len())?;
            return Ok(Value::Vault(Some(loaded)));
        }
        Source::File { path, format } => {
            sized(path)?;
            io::read_configured(path, TypeConstructor(format), *format == "Json")
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?
        }
        Source::Sqlite { path, table, order } => {
            sized(path)?;
            crate::endpoints::sqlite_rows(path, table, order, columns)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?
                .into_iter()
                .map(|row| Value::Data(Arc::new(row)))
                .collect()
        }
        Source::Postgres {
            dsn_env,
            table,
            order,
        } => {
            let dsn = dsn(configured, dsn_env).map_err(failed)?;
            crate::postgres::rows(&dsn, table, order, columns)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?
                .into_iter()
                .map(|row| Value::Data(Arc::new(row)))
                .collect()
        }
        Source::Spark {
            server,
            token_env,
            table,
            order,
        } => {
            let server = io::policy_permits_network()
                .map_err(|message| format!("endpoint `{id}`: {message}"))
                .and_then(|()| spark_server(configured, server, token_env.as_deref()))
                .map_err(failed)?;
            crate::spark::rows(&server, table, order, columns)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?
                .into_iter()
                .map(|row| Value::Data(Arc::new(row)))
                .collect()
        }
        kind @ (Source::Kafka { .. } | Source::Nats { .. } | Source::NatsJetStream { .. }) => {
            return Err(failed(live_by_source(id, kind)));
        }
    };
    within(documents.len())?;
    Ok(Value::Traversal(Arc::new(Traversal::kept(
        TypeRef::DATA,
        documents,
    ))))
}

/// A configured finite source one element at a time: a file of documents
/// read as `tap` reads the file, and a JSON array or a table, which are read
/// whole, then produced as a stream.
fn tap_configured(
    id: &str,
    vault: &crate::vault::Vault,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let configured =
        configured(id, vault).map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
    if let crate::endpoints::Kind::Kafka {
        brokers,
        topic,
        start,
        idle,
    } = &configured.kind
    {
        let reading = io::policy_permits_network()
            .and_then(|()| brokers.resolve())
            .map(|brokers| {
                Reading::log(
                    crate::kafka::messages(brokers, topic.clone(), *start, *idle),
                    TypeConstructor("Json"),
                    format!("endpoint `{id}`"),
                )
            })
            .map_err(|message| {
                Diagnostic::io_failure(span.clone(), format!("endpoint `{id}`: {message}"))
            })?;
        return Ok(live(reading, TypeRef::DATA, span.clone()));
    }
    if let crate::endpoints::Kind::Nats {
        server,
        subject,
        idle,
    } = &configured.kind
    {
        let reading = io::policy_permits_network()
            .and_then(|()| server.resolve())
            .map(|url| {
                Reading::log(
                    crate::nats::subscribed(url, subject.clone(), *idle),
                    TypeConstructor("Json"),
                    format!("endpoint `{id}`"),
                )
            })
            .map_err(|message| {
                Diagnostic::io_failure(span.clone(), format!("endpoint `{id}`: {message}"))
            })?;
        return Ok(live(reading, TypeRef::DATA, span.clone()));
    }
    if let crate::endpoints::Kind::NatsJetStream { server, held } = &configured.kind {
        let reading = io::policy_permits_network()
            .and_then(|()| server.resolve())
            .map(|url| {
                Reading::log(
                    crate::nats::messages(url, held.clone()),
                    TypeConstructor("Json"),
                    format!("endpoint `{id}`"),
                )
            })
            .map_err(|message| {
                Diagnostic::io_failure(span.clone(), format!("endpoint `{id}`: {message}"))
            })?;
        return Ok(live(reading, TypeRef::DATA, span.clone()));
    }
    if let crate::endpoints::Kind::File { path, format } = &configured.kind
        && *format != "Json"
    {
        let reading = Reading::file(path.clone(), TypeConstructor(format));
        return Ok(live(reading, TypeRef::DATA, span.clone()));
    }
    let Value::Traversal(read) = read_configured(id, vault, None, span)? else {
        return Err(Diagnostic::runtime(
            span.clone(),
            format!("endpoint `{id}` is not read one element at a time"),
        ));
    };
    let streamed = Traversal::new(TypeRef::DATA, false, crate::traversal::Source::Kept);
    streamed.keep(read.forced().map(<[Value]>::to_vec).unwrap_or_default());
    Ok(Value::Traversal(Arc::new(streamed)))
}

/// Whether a step is this extension's `source` or `tap`: a read whose
/// documents decode as the element type it is expected to give.
pub(crate) fn reads(step: &Operation) -> bool {
    matches!(step.name, "source" | "tap")
        && matches!(
            step.spec,
            Signature::Native(OperationSpec {
                output: O::Computed(_),
                ..
            })
        )
}

/// The documents a read produced, decoded as the element of `wanted`, the
/// type the checker fixed for the read: as they are when that is the
/// format's own document type, and each one read as a declared record
/// otherwise. A CSV cell is text, so it is parsed as the scalar its field
/// wants; a Json, Yaml or JsonLines leaf keeps the type it was written with.
pub(crate) fn decoded(
    vault: &crate::vault::Vault,
    endpoint: &Value,
    produced: Value,
    wanted: &TypeRef,
    arguments: &[TypedArg],
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let (Value::Traversal(traversal), Some(element)) = (&produced, wanted.args.first()) else {
        return Ok(produced);
    };
    if traversal.element == *element {
        return Ok(produced);
    }
    let (format, from) = match configured_id(endpoint) {
        Some(id) => (
            match configured(id, vault).map(|configured| &configured.kind) {
                Ok(crate::endpoints::Kind::File { format, .. }) => *format,
                _ => "Data",
            },
            format!("endpoint `{id}`"),
        ),
        None => {
            let endpoint = self::endpoint(endpoint, span)?;
            let format = if endpoint.type_of().is(&TypeRef::ROW_TRANSPORT) {
                "Data"
            } else {
                evaluated_format(arguments, &endpoint, span)?.0
            };
            (format, endpoint.describe())
        }
    };
    let leaves = if format == "Csv" {
        crate::types::Leaves::Text
    } else {
        crate::types::Leaves::AsWritten
    };
    // A configured endpoint read one element at a time, `tap("id")`, is read
    // through its boundary, which the read itself has not applied. A topic
    // always is: what `sink("id")` writes through the boundary, a `Decimal`
    // as text among it, reads back as it was written.
    let boundary = match (configured_id(endpoint), endpoint) {
        (Some(id), _) if traversal.element == TypeRef::DATA => configured(id, vault)
            .ok()
            .filter(|configured| !configured.fields.is_empty() || configured.kind.is_live())
            .map(|configured| boundary_for(configured, element, None))
            .transpose()
            .map_err(|message| Diagnostic::io_failure(span.clone(), message))?
            .flatten()
            .map(Arc::new),
        // A constructed table has no boundary of its own, and is read
        // through the one its format gives, so a column reads as its type
        // holds it: an SQLite `INTEGER` 1 as `true`, a `TEXT` as a
        // `Decimal` with its digits.
        // A column the record does not read is not read, as a configured
        // table selects only the columns its boundary reads.
        (None, Value::Endpoint(held)) => match constructed_table(held) {
            Some(mut constructed) => {
                constructed.strictness = crate::endpoints::Strictness::Open;
                boundary_for(&constructed, element, None)
                    .map_err(|message| Diagnostic::io_failure(span.clone(), message))?
                    .map(Arc::new)
            }
            None => None,
        },
        _ => None,
    };
    if let Some(values) = traversal.forced() {
        let decoded = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let value = match &boundary {
                    Some(boundary) => remapped(value, boundary)?,
                    None => value.clone(),
                };
                decode_document(&value, element, leaves, index + 1, &from)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
        return Ok(Value::Traversal(Arc::new(Traversal::kept(
            element.clone(),
            decoded,
        ))));
    }
    Ok(Value::Traversal(Arc::new(Traversal::new(
        element.clone(),
        traversal.finite,
        Source::Decode {
            input: Arc::clone(traversal),
            leaves,
            boundary,
            from,
            span: span.clone(),
        },
    ))))
}

/// One document decoded as `element`, or the error naming its position, the
/// endpoint and the field that does not decode.
pub(crate) fn decode_document(
    value: &Value,
    element: &TypeRef,
    leaves: crate::types::Leaves,
    position: usize,
    from: &str,
) -> Result<Value, String> {
    value.decoded(element, leaves).map_err(|mismatch| {
        format!("{from}: document {position} does not decode as {element}: {mismatch}")
    })
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

/// One cell of a row written through a boundary, in its external type: a
/// `Decimal` as text where its representation is text, a list or a record
/// as a tree where the format nests, and nothing for an absent field.
fn written_cell(value: Option<&Value>, external_type: &str) -> Option<crate::data::Data> {
    use crate::data::Data;
    let value = match value? {
        Value::Present(held) => held.as_ref(),
        Value::Absent(_) => return None,
        other => other,
    };
    Some(match value {
        Value::Int(n) => Data::Int(*n),
        Value::Float(x) => Data::Float(*x),
        Value::Decimal(d)
            if matches!(
                external_type.to_ascii_lowercase().as_str(),
                "string" | "text"
            ) =>
        {
            Data::Str(d.to_string())
        }
        Value::Decimal(d) => Data::Decimal(d.clone()),
        Value::Time(time) => Data::Time(*time),
        Value::Bytes(octets) => Data::Bytes(Arc::clone(octets)),
        Value::Bool(b) => Data::Bool(*b),
        Value::Str(text) => Data::Str(text.to_string()),
        Value::Data(tree) => tree.as_ref().clone(),
        // A list or a record in a format that nests is the tree it holds.
        other if matches!(external_type, "array" | "object") => {
            crate::extensions::format::piece(other, TypeConstructor("Json"), false)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .map_or_else(|| Data::Str(other.to_string()), io::data_of)
        }
        other => Data::Str(other.to_string()),
    })
}

/// The fields a boundary writes, in order: each the boundary places takes
/// its position, and the rest fill the places left in declaration order.
fn written_order(
    boundary: &crate::boundary::EffectiveBoundary,
) -> Vec<&crate::boundary::EffectiveField> {
    let written: Vec<&crate::boundary::EffectiveField> = boundary
        .fields
        .iter()
        .filter(|field| field.writable.value)
        .collect();
    let mut places: Vec<Option<&crate::boundary::EffectiveField>> = vec![None; written.len()];
    let mut rest = Vec::new();
    for field in &written {
        let place = field.position.value.checked_sub(1);
        match place {
            Some(place)
                if field.position.origin != crate::boundary::Origin::Format
                    && place < places.len()
                    && places[place].is_none() =>
            {
                places[place] = Some(field);
            }
            _ => rest.push(*field),
        }
    }
    let mut rest = rest.into_iter();
    places
        .into_iter()
        .map(|place| place.or_else(|| rest.next()))
        .collect::<Option<Vec<_>>>()
        .unwrap_or(written)
}

/// One field of a row in its external type, or `None` where the row holds
/// none.
fn row_cell(row: &Value, field: &crate::boundary::EffectiveField) -> Option<crate::data::Data> {
    use crate::data::Data;
    let held = match row {
        Value::Record(record) => record.field(&field.field.name).cloned(),
        Value::Data(tree) => match tree.as_ref() {
            Data::Map(entries) => entries
                .get(&field.field.name)
                .map(|entry| Value::Data(Arc::new(entry.clone()))),
            _ => None,
        },
        _ => None,
    };
    written_cell(held.as_ref(), &field.external_type.value)
}

/// A row as a document: each field under its external name, nested where
/// its external field is a path.
fn row_document(row: &Value, fields: &[&crate::boundary::EffectiveField]) -> Value {
    use crate::data::Data;
    use crate::endpoints::External;
    let mut entries = std::collections::BTreeMap::new();
    for field in fields {
        let Some(value) = row_cell(row, field) else {
            continue;
        };
        let path = match &field.external.value {
            External::Name(name) => vec![name.clone()],
            External::Path(segments) => segments.clone(),
        };
        let mut at = &mut entries;
        for segment in &path[..path.len() - 1] {
            let next = at
                .entry(segment.clone())
                .or_insert_with(|| Data::Map(std::collections::BTreeMap::new()));
            if !matches!(next, Data::Map(_)) {
                *next = Data::Map(std::collections::BTreeMap::new());
            }
            let Data::Map(inner) = next else {
                unreachable!("made a map above");
            };
            at = inner;
        }
        at.insert(path[path.len() - 1].clone(), value);
    }
    Value::Data(Arc::new(Data::Map(entries)))
}

/// The columns a boundary writes to a table, each its name and its
/// declared type.
fn table_columns(fields: &[&crate::boundary::EffectiveField]) -> Vec<(String, String)> {
    fields
        .iter()
        .map(|field| {
            (
                crate::boundary::written(&field.external.value),
                field.external_type.value.to_ascii_uppercase(),
            )
        })
        .collect()
}

/// A row as the text of each column a boundary writes; `None` is `NULL`.
fn postgres_row(row: &Value, fields: &[&crate::boundary::EffectiveField]) -> Vec<Option<String>> {
    fields
        .iter()
        .map(|field| {
            row_cell(row, field)
                .as_ref()
                .and_then(crate::postgres::text_of)
        })
        .collect()
}

/// The address and the token of a Spark SQL endpoint, read from the
/// environment where the configuration names a variable.
pub(crate) fn spark_server(
    configured: &crate::endpoints::Configured,
    server: &crate::endpoints::Address,
    token_env: Option<&str>,
) -> Result<crate::spark::Server, String> {
    let located = |message: String| format!("endpoint `{}`: {message}", configured.id);
    Ok(crate::spark::Server {
        url: server.resolve().map_err(located)?,
        token: token_env
            .map(|variable| crate::endpoints::secret(variable, "token_env"))
            .transpose()
            .map_err(located)?,
    })
}

/// A row as the text of each column a boundary writes to a Spark table;
/// `None` is `NULL`.
fn spark_row(row: &Value, fields: &[&crate::boundary::EffectiveField]) -> Vec<Option<String>> {
    fields
        .iter()
        .map(|field| {
            row_cell(row, field)
                .as_ref()
                .and_then(crate::spark::text_of)
        })
        .collect()
}

/// A value as the text of one message of a topic or a subject: its document
/// through the boundary where it has one, and its JSON otherwise.
fn message_text(
    value: &Value,
    fields: Option<&[&crate::boundary::EffectiveField]>,
) -> Result<String, String> {
    let document = match fields {
        Some(fields) => row_document(value, fields),
        None => value.clone(),
    };
    crate::extensions::format::piece(&document, TypeConstructor("Json"), false)
}

/// The connection string a PostgreSQL source's `dsn_env` names.
fn dsn(configured: &crate::endpoints::Configured, dsn_env: &str) -> Result<String, String> {
    crate::endpoints::secret(dsn_env, "dsn_env")
        .map_err(|message| format!("endpoint `{}`: {message}", configured.id))
}

/// `xs | sink("id")`: each value written to a configured Kafka topic as one
/// message, or to a PostgreSQL table as one row, as it arrives (HQL-0046).
fn sink_configured(
    cx: &mut dyn EvalCx,
    id: &str,
    input: Value,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    use crate::endpoints::Kind as Source;
    let failed = |message: String| Diagnostic::io_failure(span.clone(), message);
    if let Policy::Document { .. } = io::policy() {
        return Err(failed(format!(
            "endpoint `{id}`: a document writes nothing outside itself"
        )));
    }
    let configured = configured(id, cx.vault()).map_err(failed)?.clone();
    let element = match &input {
        Value::Traversal(traversal) => traversal.element.clone(),
        other => other.type_of().element().unwrap_or(TypeRef::DATA),
    };
    let boundary = boundary_for(&configured, &element, None).map_err(failed)?;
    let fields = boundary.as_ref().map(written_order);
    let each = |cx: &mut dyn EvalCx,
                write: &mut dyn FnMut(Value) -> Result<(), String>|
     -> Result<(), Diagnostic> {
        match &input {
            Value::Traversal(traversal) => cx
                .traverse(traversal, &mut |_, value| {
                    write(value)
                        .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
                    Ok(true)
                })
                .map(drop),
            other => {
                for value in other.elements().unwrap_or_default() {
                    write(value)
                        .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
                }
                Ok(())
            }
        }
    };
    match &configured.kind {
        Source::Kafka { brokers, topic, .. } => {
            let brokers = brokers
                .resolve()
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            let mut producer = crate::kafka::Producer::open(&brokers, topic)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            each(cx, &mut |value| {
                producer.send(message_text(&value, fields.as_deref())?)
            })?;
            producer
                .flush()
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
        }
        Source::Nats {
            server, subject, ..
        }
        | Source::NatsJetStream {
            server,
            held: crate::endpoints::Subject { subject, .. },
        } => {
            let stored = matches!(configured.kind, Source::NatsJetStream { .. });
            // Refused before the address is read, as before a connection.
            crate::nats::one_subject(subject)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            let url = server
                .resolve()
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            let mut publisher = crate::nats::Publisher::open(&url, subject, stored)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            each(cx, &mut |value| {
                publisher.send(message_text(&value, fields.as_deref())?)
            })?;
            publisher
                .flush()
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
        }
        Source::Spark {
            server,
            token_env,
            table,
            ..
        } => {
            let fields = fields.ok_or_else(|| {
                failed(format!("`sink(\"{id}\")` writes rows of a declared record"))
            })?;
            let server = spark_server(&configured, server, token_env.as_deref()).map_err(failed)?;
            let mut writer = crate::spark::Writer::open(
                &server,
                table,
                configured.mode,
                &table_columns(&fields),
            )
            .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            each(cx, &mut |value| writer.insert(spark_row(&value, &fields)))?;
            writer
                .flush()
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
        }
        Source::Postgres { dsn_env, table, .. } => {
            let fields = fields.ok_or_else(|| {
                failed(format!("`sink(\"{id}\")` writes rows of a declared record"))
            })?;
            let dsn = dsn(&configured, dsn_env).map_err(failed)?;
            let mut writer = crate::postgres::Writer::open(
                &dsn,
                table,
                configured.mode,
                &table_columns(&fields),
            )
            .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            each(cx, &mut |value| {
                writer.insert(&postgres_row(&value, &fields))
            })?;
        }
        other => {
            return Err(failed(format!(
                "endpoint `{id}` is a `{}` endpoint, which `sink` does not write; write it whole \
                 with `target`",
                other.name()
            )));
        }
    }
    Ok(Value::Unit)
}

/// `xs | target("id")`: a `Table<R>` or `Seq<R>` written to a configured
/// target through its boundary, as its `mode` says (HQL-0035).
fn write_configured(
    cx: &mut dyn EvalCx,
    id: &str,
    input: Value,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    let configured = configured(id, cx.vault())
        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?
        .clone();
    write_to(cx, &configured, input, span)
}

/// `insert E rows` (HQL-0051): the rows added to the table the endpoint
/// names, after the ones it holds, in one statement of its system. A tree
/// is read as the rows' declared type; a table that does not exist is a
/// failure, since a mutation changes rows that exist and `target` creates
/// a table. The value is the number of rows added.
pub(crate) fn insert_rows(
    cx: &mut dyn EvalCx,
    configured: &crate::endpoints::Configured,
    row: &TypeRef,
    input: Value,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    use crate::endpoints::{Kind as Source, WriteMode};
    let failed = |message: String| Diagnostic::io_failure(span.clone(), message);
    let id = configured.id.as_str();
    let (element, rows) = match &input {
        Value::Table(table) => (table.row.clone(), table.rows.clone()),
        Value::Traversal(traversal) => {
            let mut values = Vec::new();
            cx.traverse(traversal, &mut |_, value| {
                values.push(value);
                Ok(true)
            })?;
            (traversal.element.clone(), values)
        }
        other if other.type_of().is_collection() => (
            other.type_of().element().unwrap_or(TypeRef::DATA),
            other.elements().unwrap_or_default(),
        ),
        one => (one.type_of(), vec![one.clone()]),
    };
    // A tree is read as the row the table holds.
    let element = if *row == TypeRef::DATA {
        element
    } else {
        row.clone()
    };
    let rows = rows
        .into_iter()
        .enumerate()
        .map(|(index, value)| match value {
            Value::Data(_) if element != TypeRef::DATA => decode_document(
                &value,
                &element,
                crate::types::Leaves::AsWritten,
                index + 1,
                &format!("endpoint `{id}`"),
            )
            .map_err(failed),
            other => Ok(other),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let count = i64::try_from(rows.len()).unwrap_or(i64::MAX);
    let missing = |table: &str| {
        failed(format!(
            "endpoint `{id}`: the table `{table}` does not exist; `insert` adds rows to a table,              and `target` creates one"
        ))
    };
    match &configured.kind {
        Source::Sqlite { path, table, .. } => {
            let columns = crate::endpoints::sqlite_columns(path, table).unwrap_or_default();
            if columns.is_empty() {
                return Err(missing(table));
            }
        }
        Source::Postgres { dsn_env, table, .. } => {
            let dsn = dsn(configured, dsn_env).map_err(failed)?;
            crate::postgres::columns(&dsn, table).map_err(|_| missing(table))?;
        }
        Source::Spark {
            server,
            token_env,
            table,
            ..
        } => {
            let server = spark_server(configured, server, token_env.as_deref()).map_err(failed)?;
            crate::spark::columns(&server, table).map_err(|_| missing(table))?;
        }
        other => {
            return Err(failed(format!(
                "`insert` adds rows to a table of a database, and endpoint `{id}` is a `{}`                  endpoint",
                other.name()
            )));
        }
    }
    let mut appended = configured.clone();
    appended.mode = WriteMode::Append;
    write_to(cx, &appended, Value::list(rows, element), span)?;
    Ok(Value::Int(count))
}

/// A `Table<R>` or `Seq<R>` written to an endpoint as its configuration
/// says: one configured under an id, or a transport constructed in the
/// program, which writes as its kind does with no boundary of its own.
fn write_to(
    cx: &mut dyn EvalCx,
    configured: &crate::endpoints::Configured,
    input: Value,
    span: &Range<usize>,
) -> Result<Value, Diagnostic> {
    use crate::data::Data;
    use crate::endpoints::{Kind as Source, WriteMode};
    let failed = |message: String| Diagnostic::io_failure(span.clone(), message);
    let id = configured.id.as_str();
    let configured = configured.clone();
    let (element, rows) = match &input {
        Value::Table(table) => (table.row.clone(), table.rows.clone()),
        Value::Traversal(traversal) => {
            let mut values = Vec::new();
            cx.traverse(traversal, &mut |_, value| {
                values.push(value);
                Ok(true)
            })?;
            (traversal.element.clone(), values)
        }
        other => (
            other.type_of().element().unwrap_or(TypeRef::DATA),
            other.elements().unwrap_or_default(),
        ),
    };
    if configured.kind.is_live() {
        return sink_configured(cx, id, Value::list(rows, element), span);
    }
    let boundary = boundary_for(&configured, &element, None)
        .map_err(failed)?
        .ok_or_else(|| {
            failed(format!(
                "`target(\"{id}\")` writes rows of a declared record"
            ))
        })?;
    let fields = written_order(&boundary);
    let cell = row_cell;
    let document = |row: &Value| row_document(row, &fields);
    let mode = configured.mode;
    match &configured.kind {
        Source::Vault { .. } => Err(failed(format!(
            "endpoint `{id}` is a vault, which is read and never written"
        ))),
        Source::Kafka { .. } | Source::Nats { .. } | Source::NatsJetStream { .. } => {
            unreachable!("a topic or a subject is written above")
        }
        Source::Spark {
            server,
            token_env,
            table,
            ..
        } => {
            if let Policy::Document { .. } = io::policy() {
                return Err(failed(format!(
                    "endpoint `{id}`: a document writes nothing outside itself"
                )));
            }
            let server = spark_server(&configured, server, token_env.as_deref()).map_err(failed)?;
            let values: Vec<Vec<Option<String>>> =
                rows.iter().map(|row| spark_row(row, &fields)).collect();
            crate::spark::write(&server, table, mode, &table_columns(&fields), &values)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            Ok(Value::Unit)
        }
        Source::Postgres { dsn_env, table, .. } => {
            if let Policy::Document { .. } = io::policy() {
                return Err(failed(format!(
                    "endpoint `{id}`: a document writes nothing outside itself"
                )));
            }
            let dsn = dsn(&configured, dsn_env).map_err(failed)?;
            let values: Vec<Vec<Option<String>>> =
                rows.iter().map(|row| postgres_row(row, &fields)).collect();
            crate::postgres::write(&dsn, table, mode, &table_columns(&fields), &values)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            Ok(Value::Unit)
        }
        Source::Sqlite { path, table, .. } => {
            if let Policy::Document { .. } = io::policy() {
                return Err(failed(format!(
                    "endpoint `{id}`: a document writes nothing outside itself"
                )));
            }
            let columns: Vec<(String, String)> = fields
                .iter()
                .map(|field| {
                    (
                        crate::boundary::written(&field.external.value),
                        field.external_type.value.to_ascii_uppercase(),
                    )
                })
                .collect();
            let values: Vec<Vec<rusqlite::types::Value>> = rows
                .iter()
                .map(|row| {
                    fields
                        .iter()
                        .map(|field| match cell(row, field) {
                            None | Some(Data::Empty) => rusqlite::types::Value::Null,
                            Some(Data::Int(n)) => rusqlite::types::Value::Integer(n),
                            Some(Data::Bool(b)) => rusqlite::types::Value::Integer(i64::from(b)),
                            Some(Data::Float(x)) => rusqlite::types::Value::Real(x),
                            // A decimal keeps its digits as text; it is a REAL
                            // only in a column the boundary permits to lose them,
                            // and an INTEGER only where it is whole.
                            Some(Data::Decimal(d)) => {
                                match field.external_type.value.to_ascii_uppercase().as_str() {
                                    "REAL" | "NUMERIC" if field.lossy.value => {
                                        d.to_float().map_or_else(
                                            || rusqlite::types::Value::Text(d.to_string()),
                                            rusqlite::types::Value::Real,
                                        )
                                    }
                                    "INTEGER" if d.scale() <= 0 => d.to_int().map_or_else(
                                        || rusqlite::types::Value::Text(d.to_string()),
                                        rusqlite::types::Value::Integer,
                                    ),
                                    _ => rusqlite::types::Value::Text(d.to_string()),
                                }
                            }
                            Some(Data::Str(text)) => rusqlite::types::Value::Text(text),
                            Some(other) => rusqlite::types::Value::Text(other.to_string()),
                        })
                        .collect()
                })
                .collect();
            crate::endpoints::sqlite_write(path, table, mode, &columns, &values)
                .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
            Ok(Value::Unit)
        }
        Source::File { path, format } => {
            let format = TypeConstructor(format);
            let exists = path.exists();
            if mode == WriteMode::Create && exists {
                return Err(failed(format!(
                    "endpoint `{id}`: {} exists, and `mode = \"create\"` writes only a new file",
                    path.display()
                )));
            }
            let documents: Vec<Value> = rows.iter().map(document).collect();
            let appending = mode == WriteMode::Append;
            let held = exists && std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0);
            let rendered = |values: Vec<Value>| -> Result<String, Diagnostic> {
                let text = match format.0 {
                    "Csv" => {
                        let columns = fields
                            .iter()
                            .map(|field| crate::boundary::written(&field.external.value))
                            .collect();
                        let table = crate::types::Table::new(values, Some(columns), TypeRef::DATA)
                            .map_err(failed)?;
                        crate::extensions::format::render(
                            &Value::Table(Arc::new(table)),
                            format,
                            0,
                            None,
                            &cx.vault().formats,
                        )
                    }
                    "Json" => crate::extensions::format::render(
                        &Value::list(values, TypeRef::DATA),
                        format,
                        0,
                        None,
                        &cx.vault().formats,
                    ),
                    _ => values
                        .iter()
                        .map(|value| crate::extensions::format::piece(value, format, false))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|pieces| pieces.join("\n")),
                }
                .map_err(failed)?;
                // One line ending throughout, as a sink's pieces have it.
                Ok(text.lines().collect::<Vec<_>>().join("\n"))
            };
            match (format.0, appending) {
                // A JSON file is one array: appending extends it and writes
                // it again.
                ("Json", true) if held => {
                    let mut held = io::read_configured(path, format, true)
                        .map_err(|message| failed(format!("endpoint `{id}`: {message}")))?;
                    held.extend(documents);
                    let text = rendered(held)?;
                    io::write_whole(&Endpoint::File(path.clone()), &text).map_err(failed)?;
                }
                (_, true) => {
                    let mut text = rendered(documents)?;
                    // A CSV file already has its header.
                    if format.0 == "Csv" && held {
                        text = text.lines().skip(1).collect::<Vec<_>>().join("\n");
                    }
                    if !text.is_empty() {
                        let (mut destination, _) =
                            Destination::open(&Endpoint::FileEnd(path.clone())).map_err(failed)?;
                        destination.line(&text).map_err(failed)?;
                    }
                }
                (_, false) => {
                    let text = rendered(documents)?;
                    io::write_whole(&Endpoint::File(path.clone()), &text).map_err(failed)?;
                }
            }
            Ok(Value::Unit)
        }
    }
}

fn eval_target(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument = arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some("endpoint"))
        .or_else(|| arguments.iter().find(|argument| argument.name.is_none()));
    if let Some(argument) = argument
        && let TypedKind::Str(id) = &argument.value.kind
    {
        return write_configured(cx, id, input, &span);
    }
    let endpoint = written_to(cx, arguments, &span)?;
    if let Some(id) = endpoint.configured_id() {
        return write_configured(cx, id, input, &span);
    }
    if let Some(constructed) = constructed_table(&endpoint) {
        return write_to(cx, &constructed, input, &span);
    }
    if let Endpoint::Channel(name, _) = endpoint.as_ref() {
        io::push(name, settled(cx, input)?)
            .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
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
        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
    match endpoint.as_ref() {
        Endpoint::FileEnd(_) => {
            let (mut destination, _) = Destination::open(&endpoint)
                .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
            destination.line(text.trim_end_matches('\n'))
        }
        _ => io::write_whole(&endpoint, text.trim_end_matches('\n')),
    }
    .map_err(|message| Diagnostic::io_failure(span, message))?;
    Ok(Value::Unit)
}

fn eval_sink(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let argument = arguments
        .iter()
        .find(|argument| argument.name.as_deref() == Some("endpoint"))
        .or_else(|| arguments.iter().find(|argument| argument.name.is_none()));
    if let Some(argument) = argument
        && let TypedKind::Str(id) = &argument.value.kind
    {
        return sink_configured(cx, id, input, &span);
    }
    let endpoint = written_to(cx, arguments, &span)?;
    if let Some(id) = endpoint.configured_id() {
        return sink_configured(cx, id, input, &span);
    }
    if let Endpoint::Channel(name, _) = endpoint.as_ref() {
        match &input {
            Value::Traversal(traversal) => {
                cx.traverse(traversal, &mut |_, value| {
                    io::push(name, value)
                        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
                    Ok(true)
                })?;
            }
            other => {
                for value in other.elements().unwrap_or_default() {
                    io::push(name, value)
                        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
                }
            }
        }
        return Ok(Value::Unit);
    }
    let format = evaluated_format(arguments, &endpoint, &span)?;
    let (mut destination, held) = Destination::open(&endpoint)
        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
    // A CSV header is written once: not again when appending to rows.
    let mut first = !held;
    let mut write = |value: Value| -> Result<bool, Diagnostic> {
        let text = crate::extensions::format::piece(&value, format, first)
            .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
        first = false;
        destination
            .line(&text)
            .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
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
    destination
        .finish()
        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
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
