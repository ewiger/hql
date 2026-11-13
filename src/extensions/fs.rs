//! The module `io.fs`: folders, globs and the entries a folder holds.
//!
//! Constructing a `Folder` or a `Glob`, and matching a glob, read text and
//! nothing else. Listing reads the file system once, where it is written,
//! through the sorted walk the vault reads its documents with
//! ([`crate::walk`]), held to the grant a file read is held to, and gives a
//! held `Seq<Entry>`. See [HQL-0040](../../doc/proposals/HQL-0040/README.md).

use super::io::{path_text, path_value};
use super::{EvalCx, Operation, Purity, Shape, Signature, declared};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::glob::{self, Glob};
use crate::io::{self, Listing};
use crate::traversal::Traversal;
use crate::types::{Record, TypeRef, Value};
use crate::walk::{self, Kind, Links, Listed, Walk, paths};
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

/// An operation that reads the file system.
const fn listing(name: &'static str, eval: super::Eval) -> Operation {
    Operation {
        name,
        spec: Signature::Declared,
        purity: Purity::Effectful,
        shape: Shape::Barrier,
        check: None,
        eval,
    }
}

/// Every operation `io.fs` provides.
pub(crate) static STEPS: &[Operation] = &[
    declared("folder", eval_folder),
    declared("Glob", eval_glob_of),
    declared("matches", eval_matches),
    listing("entries", eval_entries),
    listing("walk", eval_walk),
    listing("glob", eval_glob),
    listing("exists", eval_exists),
    listing("entry", eval_entry),
];

/// How `io.fs` lists: every entry, hidden ones included, and a link
/// reported rather than followed.
const LISTED: Walk = Walk {
    links: Links::Report,
    hidden: true,
};

type Result<T> = std::result::Result<T, Diagnostic>;

fn fail(span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
    Diagnostic::runtime(span.clone(), message)
}

fn record(ty: TypeRef, fields: Vec<(&str, Value)>) -> Value {
    Value::Record(Arc::new(Record {
        ty,
        fields: fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
        carrier: None,
    }))
}

fn text(text: &str) -> Value {
    Value::Str(Arc::from(text))
}

/// The field `name` of a record of type `ty`.
fn field(
    value: &Value,
    ty: &TypeRef,
    name: &str,
    step: &str,
    span: &Range<usize>,
) -> Result<Value> {
    match value {
        Value::Present(held) => field(held, ty, name, step, span),
        Value::Record(record) if record.ty == *ty => record
            .field(name)
            .cloned()
            .ok_or_else(|| fail(span, format!("`{step}` takes a {ty}"))),
        other => Err(fail(
            span,
            format!("`{step}` takes a {ty}, not {}", other.type_of()),
        )),
    }
}

/// The text of a `Folder`'s path.
fn folder_text(value: &Value, step: &str, span: &Range<usize>) -> Result<String> {
    path_text(
        &field(value, &TypeRef::FOLDER, "path", step, span)?,
        step,
        span,
    )
}

/// The compiled pattern of a `Glob`.
fn compiled(value: &Value, step: &str, span: &Range<usize>) -> Result<Glob> {
    match field(value, &TypeRef::GLOB, "pattern", step, span)? {
        Value::Str(pattern) => Glob::parse(&pattern).map_err(|message| fail(span, message)),
        other => Err(fail(
            span,
            format!("a Glob's pattern is a String, not {}", other.type_of()),
        )),
    }
}

/// The value of the argument written `name` or first without a name.
fn argument(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    name: &str,
    step: &str,
    span: &Range<usize>,
) -> Result<Value> {
    match super::named(arguments, name)
        .or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
    {
        Some(argument) => cx.evaluate(&argument.value),
        None => Err(fail(span, format!("`{step}` takes `{name}`"))),
    }
}

fn eval_folder(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let path = match input {
        Value::Str(path) => path_value(&path),
        other => path_value(&path_text(&other, "folder", &span)?),
    };
    Ok(record(TypeRef::FOLDER, vec![("path", path)]))
}

fn eval_glob_of(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let Value::Str(pattern) = input else {
        return Err(fail(
            &span,
            format!("`Glob` takes a String, not {}", input.type_of()),
        ));
    };
    Glob::parse(&pattern).map_err(|message| fail(&span, message))?;
    Ok(record(
        TypeRef::GLOB,
        vec![("pattern", Value::Str(pattern))],
    ))
}

fn eval_matches(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let glob = compiled(&input, "matches", &span)?;
    let path = argument(cx, arguments, "path", "matches", &span)?;
    let path = path_text(&path, "matches", &span)?;
    Ok(Value::Bool(glob.matches(&glob::segments(&path))))
}

/// The segments of a path below the folder walked, as text.
fn segments(relative: &Path) -> Vec<String> {
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect()
}

/// The `Entry` a listing reached below the folder at `folder`.
fn entry_value(folder: &str, listed: &Listed) -> Value {
    let below = segments(&listed.relative);
    let path = below.iter().fold(folder.to_owned(), |path, segment| {
        paths::join(&path, segment)
    });
    entry_at(&path, listed.kind, listed.size)
}

fn entry_at(path: &str, kind: Kind, size: Option<u64>) -> Value {
    let size = match size {
        Some(bytes) => Value::Present(Arc::new(Value::Int(
            i64::try_from(bytes).unwrap_or(i64::MAX),
        ))),
        None => Value::Absent(TypeRef::INT),
    };
    record(
        TypeRef::ENTRY,
        vec![
            ("path", path_value(path)),
            ("name", text(paths::name(path))),
            ("kind", text(kind.label())),
            ("size", size),
        ],
    )
}

/// A listing's entries, held, so every traversal of it sees the same ones.
fn held(entries: Vec<Value>) -> Value {
    Value::Traversal(Arc::new(Traversal::kept(TypeRef::ENTRY, entries)))
}

/// The folder a listing reads, granted and present.
fn listed_folder(folder: &str, span: &Range<usize>) -> Result<std::path::PathBuf> {
    let full = io::listable(Path::new(folder), Listing::Folder)
        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
    match walk::stat(&full, Links::Follow) {
        Ok(Some((Kind::Folder, _))) => Ok(full),
        Ok(Some(_)) => Err(Diagnostic::io_failure(
            span.clone(),
            format!("{folder}: not a folder"),
        )),
        Ok(None) => Err(Diagnostic::io_failure(
            span.clone(),
            format!("{folder}: no folder is there"),
        )),
        Err(error) => Err(Diagnostic::io_failure(
            span.clone(),
            format!("{folder}: {error}"),
        )),
    }
}

/// A listing held within the run context's bound on what one evaluation
/// holds of an endpoint: more is an error, never a shorter listing.
struct Bounded<'a> {
    folder: &'a str,
    most: Option<usize>,
    entries: Vec<Value>,
}

impl Bounded<'_> {
    fn push(&mut self, entry: Value, span: &Range<usize>) -> Result<()> {
        if let Some(most) = self.most
            && self.entries.len() >= most
        {
            return Err(Diagnostic::io_failure(
                span.clone(),
                format!(
                    "{}: the listing holds more than `max_elements = {most}` entries",
                    self.folder
                ),
            ));
        }
        self.entries.push(entry);
        Ok(())
    }
}

/// Every entry below `folder` the filter keeps, in walk order.
fn walked(
    cx: &dyn EvalCx,
    folder: &str,
    keep: &dyn Fn(&Listed) -> bool,
    span: &Range<usize>,
) -> Result<Value> {
    let full = listed_folder(folder, span)?;
    let mut bounded = Bounded {
        folder,
        most: cx.vault().endpoints.bounds.max_elements,
        entries: Vec::new(),
    };
    walk::walk::<Failure>(&full, LISTED, &mut |listed| {
        if keep(&listed) {
            bounded
                .push(entry_value(folder, &listed), span)
                .map_err(Failure::Bounded)?;
        }
        Ok(true)
    })
    .map_err(|failure| match failure {
        Failure::Read(error) => Diagnostic::io_failure(span.clone(), format!("{folder}: {error}")),
        Failure::Bounded(diagnostic) => diagnostic,
    })?;
    Ok(held(bounded.entries))
}

/// Why a walk stopped short.
enum Failure {
    Read(std::io::Error),
    Bounded(Diagnostic),
}

impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Self::Read(error)
    }
}

fn eval_entries(
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let folder = folder_text(&input, "entries", &span)?;
    let full = listed_folder(&folder, &span)?;
    let listed = walk::entries(&full, LISTED)
        .map_err(|error| Diagnostic::io_failure(span.clone(), format!("{folder}: {error}")))?;
    let mut bounded = Bounded {
        folder: &folder,
        most: cx.vault().endpoints.bounds.max_elements,
        entries: Vec::new(),
    };
    for listed in &listed {
        bounded.push(entry_value(&folder, listed), &span)?;
    }
    Ok(held(bounded.entries))
}

fn eval_walk(
    cx: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let folder = folder_text(&input, "walk", &span)?;
    walked(cx, &folder, &|_| true, &span)
}

fn eval_glob(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let folder = folder_text(&input, "glob", &span)?;
    let pattern = argument(cx, arguments, "pattern", "glob", &span)?;
    let glob = compiled(&pattern, "glob", &span)?;
    walked(
        cx,
        &folder,
        &|listed| glob.matches(&segments(&listed.relative)),
        &span,
    )
}

/// What is at a path, granted as a listing of it is: a link reported.
fn stat_at(path: &str, span: &Range<usize>) -> Result<Option<(Kind, Option<u64>)>> {
    let full = io::listable(Path::new(path), Listing::Entry)
        .map_err(|message| Diagnostic::io_failure(span.clone(), message))?;
    walk::stat(&full, Links::Report)
        .map_err(|error| Diagnostic::io_failure(span.clone(), format!("{path}: {error}")))
}

fn eval_exists(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let path = path_text(&input, "exists", &span)?;
    Ok(Value::Bool(stat_at(&path, &span)?.is_some()))
}

fn eval_entry(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let path = path_text(&input, "entry", &span)?;
    Ok(match stat_at(&path, &span)? {
        Some((kind, size)) => Value::Present(Arc::new(entry_at(&path, kind, size))),
        None => Value::Absent(TypeRef::ENTRY),
    })
}
