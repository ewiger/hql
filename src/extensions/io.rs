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
    variadic: false,
};

const FORMAT_PARAMETER: TypeParameter = TypeParameter {
    name: "F",
    bound: Some(crate::types::builtin::FORMAT),
};

const ENDPOINT: ArgumentSpec =
    ArgumentSpec::value("endpoint", P::Type(TypeConstructor("Writable")));

/// Every operation the `io` extension provides.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "file",
        spec: Signature::Native(OperationSpec {
            parameters: &[],
            input: P::Type(TypeConstructor("String")),
            arguments: &[],
            output: O::Type(P::Type(TypeConstructor("File"))),
        }),
        summary: "The file at a path, as an endpoint: read whole, written by replacing it; \
                  its `.end` is appended to and followed.",
        purity: Purity::Pure,
        shape: super::Shape::Barrier,
        check: Some(check_file),
        eval: eval_file,
    },
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
        summary: "Read an endpoint's documents in a format: every one, as a Seq, when it can \
                  be read again; each as it arrives, as a Stream, when it cannot. Given an \
                  id, the source the run context configures under it.",
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
        summary: "Read an endpoint's documents one at a time, as a Stream; at a file's \
                  `.end`, each one appended from now on.",
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
            arguments: &[ENDPOINT, FORMAT],
            output: O::Type(P::Type(TypeConstructor("Unit"))),
        }),
        summary: "Write one completed value as one document; a file is replaced, its `.end` \
                  appended to.",
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
            arguments: &[ENDPOINT, FORMAT],
            output: O::Type(P::Type(TypeConstructor("Unit"))),
        }),
        summary: "Write each value of a stream as one document, as it arrives; a file is \
                  replaced, its `.end` appended to.",
        purity: Purity::Effectful,
        shape: super::Shape::Barrier,
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
    if call.input.is(&TypeRef::STDIN)
        && let Some(format) = io::stdin_format()
    {
        return Ok(format);
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

/// The type `source` or `tap` gives.
pub(crate) fn result(
    name: &str,
    call: &CheckedCall<'_>,
    vault: &crate::vault::Vault,
) -> Result<TypeRef, Diagnostic> {
    if call.input.is(&TypeRef::STR) {
        let read = configured_type(call, vault)?;
        if name != "tap" {
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
) -> Result<TypeRef, Diagnostic> {
    let span = call
        .input_expr
        .map_or(0..0, |expression| expression.span.clone());
    let Some(Kind::Str(id)) = call.input_expr.map(|expression| &expression.kind) else {
        return Err(Diagnostic::typing(
            span,
            "`source` names a configured source by a literal id, `source(\"field_notes\")`: \
             a computed id would leave its type unknown until evaluation",
        ));
    };
    if call.bindings.contains_key("F") {
        return Err(Diagnostic::typing(
            span,
            "a configured source's format is its kind's, set in its configuration; name none",
        ));
    }
    let configured = configured(id, vault).map_err(|message| {
        if crate::sources::is_id(id) {
            Diagnostic::name(span.clone(), message)
        } else {
            Diagnostic::typing(span.clone(), message)
        }
    })?;
    if matches!(configured.kind, crate::sources::Kind::Vault { .. }) {
        return Ok(TypeRef::VAULT);
    }
    let element = match &configured.element {
        Some(written) => crate::types::named(written, &[]).ok_or_else(|| {
            Diagnostic::typing(
                span.clone(),
                format!("source `{id}` names the element type `{written}`, which is not a type"),
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

/// The configuration of the source an id names: the id must be one, and
/// configured.
fn configured<'v>(
    id: &str,
    vault: &'v crate::vault::Vault,
) -> Result<&'v crate::sources::Configured, String> {
    if !crate::sources::is_id(id) {
        return Err(
            if id.contains(':') || id.contains('/') || id.contains('.') {
                format!(
                    "{id:?} is not a source id: connection details belong in configuration, \
                 `[sources.<id>]` in hql.toml, and a program names the id"
                )
            } else {
                format!("{id:?} is not a source id: an id is snake_case, `[a-z][a-z0-9_]*`")
            },
        );
    }
    vault.sources.get(id).ok_or_else(|| {
        let hint = vault.sources.nearest(id).map_or_else(
            || "; a source is declared under `[sources.<id>]` in the vault's hql.toml".to_owned(),
            |near| format!("; did you mean `{near}`?"),
        );
        format!("no source `{id}` is configured{hint}")
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
    if matches!(endpoint, Endpoint::Stdin)
        && let Some(format) = io::stdin_format()
    {
        return Ok(format);
    }
    endpoint
        .path()
        .and_then(io::format_of_extension)
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "no format is named or implied"))
}

fn eval_source(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Value::Str(id) = &input {
        return read_configured(id, cx.vault(), None, &span);
    }
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
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    if let Value::Str(id) = &input {
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
        Endpoint::File(path) => Reading::file(path.clone(), format),
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

/// The effective boundary of a configured source read as `element`, checked
/// against `external` where the format knows its fields; `None` where the
/// element is not a record.
///
/// # Errors
/// Every boundary error, one per line.
pub(crate) fn boundary_for(
    configured: &crate::sources::Configured,
    element: &TypeRef,
    external: Option<&[String]>,
) -> Result<Option<crate::boundary::EffectiveBoundary>, String> {
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
        },
        external,
    )
    .map(Some)
    .map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
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
    use crate::sources::{Absence, External};
    let Value::Data(tree) = document else {
        return Ok(document.clone());
    };
    let Data::Map(entries) = tree.as_ref() else {
        return Ok(document.clone());
    };
    let mut mapped = entries.clone();
    for field in &boundary.fields {
        let name = &field.field.name;
        if let External::Name(external) = &field.external.value
            && external != name
        {
            mapped.remove(external);
        }
        mapped.remove(name);
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
                    "source `{}`, field `{name}`: it holds null, and its boundary reads only {} \
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
) -> Result<Value, Diagnostic> {
    use crate::sources::Kind as Source;
    let failed = |message: String| Diagnostic::runtime(span.clone(), message);
    let configured = configured(id, vault).map_err(failed)?;
    let element = wanted.args.first().cloned().unwrap_or(TypeRef::DATA);
    let endpoint = Value::Str(id.into());
    let plain = |columns: Option<&[String]>| read_configured(id, vault, columns, span);
    let boundary = match &configured.kind {
        Source::Vault { .. } => None,
        Source::Sqlite { path, table, .. } => {
            let columns = crate::sources::sqlite_columns(path, table)
                .map_err(|message| failed(format!("source `{id}`: {message}")))?;
            boundary_for(configured, &element, Some(&columns)).map_err(failed)?
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
            crate::sources::External::Name(name) => Some(name.clone()),
            crate::sources::External::Path(_) => None,
        })
        .collect();
    let columns = matches!(configured.kind, Source::Sqlite { .. }).then_some(read.as_slice());
    let Value::Traversal(produced) = plain(columns)? else {
        return Err(failed(format!(
            "source `{id}` is not a sequence of elements"
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
    let from = format!("source `{id}`");
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

/// A configured source's contents: a vault source's vault, or every element
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
    use crate::sources::Kind as Source;
    let failed = |message: String| Diagnostic::runtime(span.clone(), message);
    let configured = configured(id, vault).map_err(failed)?;
    let bounds = vault.sources.bounds;
    let within = |count: usize| match bounds.max_elements {
        Some(most) if count > most => Err(failed(format!(
            "source `{id}` holds {count} elements, more than `max_elements = {most}`; take it \
             in portions"
        ))),
        _ => Ok(()),
    };
    let sized = |path: &Path| match bounds.max_bytes {
        Some(most) => match std::fs::metadata(path) {
            Ok(metadata) if metadata.len() > most => Err(failed(format!(
                "source `{id}` is {} bytes, more than `max_bytes = {most}`",
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
                .map_err(|message| failed(format!("source `{id}`: {message}")))?
        }
        Source::Sqlite { path, table, order } => {
            sized(path)?;
            crate::sources::sqlite_rows(path, table, order, columns)
                .map_err(|message| failed(format!("source `{id}`: {message}")))?
                .into_iter()
                .map(|row| Value::Data(Arc::new(row)))
                .collect()
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
        configured(id, vault).map_err(|message| Diagnostic::runtime(span.clone(), message))?;
    if let crate::sources::Kind::File { path, format } = &configured.kind
        && *format != "Json"
    {
        let reading = Reading::file(path.clone(), TypeConstructor(format));
        return Ok(live(reading, TypeRef::DATA, span.clone()));
    }
    let Value::Traversal(read) = read_configured(id, vault, None, span)? else {
        return Err(Diagnostic::runtime(
            span.clone(),
            format!("source `{id}` is not read one element at a time"),
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
    let (format, from) = match endpoint {
        Value::Str(id) => (
            match configured(id, vault).map(|configured| &configured.kind) {
                Ok(crate::sources::Kind::File { format, .. }) => *format,
                _ => "Data",
            },
            format!("source `{id}`"),
        ),
        _ => {
            let endpoint = self::endpoint(endpoint, span)?;
            (
                evaluated_format(arguments, &endpoint, span)?.0,
                endpoint.describe(),
            )
        }
    };
    let leaves = if format == "Csv" {
        crate::types::Leaves::Text
    } else {
        crate::types::Leaves::AsWritten
    };
    // A configured source read one element at a time, `tap("id")`, is read
    // through its boundary, which the read itself has not applied.
    let boundary = match endpoint {
        Value::Str(id) if traversal.element == TypeRef::DATA => configured(id, vault)
            .ok()
            .filter(|configured| !configured.fields.is_empty())
            .map(|configured| boundary_for(configured, element, None))
            .transpose()
            .map_err(|message| Diagnostic::runtime(span.clone(), message))?
            .flatten()
            .map(Arc::new),
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
            .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
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

fn eval_target(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let endpoint = written_to(cx, arguments, &span)?;
    if let Endpoint::Channel(name, _) = endpoint.as_ref() {
        io::push(name, settled(cx, input)?)
            .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
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
                    io::push(name, value)
                        .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
                    Ok(true)
                })?;
            }
            other => {
                for value in other.elements().unwrap_or_default() {
                    io::push(name, value)
                        .map_err(|message| Diagnostic::runtime(span.clone(), message))?;
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
