//! The module `io.process` (HQL-0064, the companion note on child
//! processes): `command`, which builds a [`CommandSpec`] and starts
//! nothing, and `run`, which starts it and gives a `Result`.
//!
//! `source`, `tap`, `target` and `sink` read and write a command as they do
//! any endpoint, in [`crate::io`]. The host fixes, before evaluation, which
//! programs a run may start, the `commands` grant of a project's
//! `.hql/config.toml`, and what a child's text that is not UTF-8 becomes,
//! its `[command_output] invalid_utf8`.

use super::spec::CheckedCall;
use super::{CheckCx, EvalCx, Operation, Purity, Shape, Signature};
use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::io::{CommandSpec, Endpoint, Policy};
use crate::types::{Record, TypeRef, Value};
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Every operation `io.process` provides.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "command",
        spec: Signature::Declared,
        purity: Purity::Pure,
        shape: Shape::Barrier,
        check: Some(refuse_in_documents),
        eval: eval_command,
    },
    Operation {
        name: "run",
        spec: Signature::Declared,
        purity: Purity::Effectful,
        shape: Shape::Barrier,
        check: None,
        eval: eval_run,
    },
];

/// Which programs the host grants a run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Grant {
    /// Every program: a run whose project states no `commands`.
    #[default]
    All,
    /// The programs `commands = [...]` lists, matched as written.
    Only(Vec<String>),
}

/// What a child's output that is not UTF-8 becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InvalidUtf8 {
    /// A failure: `invalid-text` under `run`, the format's under a read.
    #[default]
    Fail,
    /// Each invalid sequence becomes U+FFFD, and the read goes on.
    Replace,
}

impl InvalidUtf8 {
    /// The setting `invalid_utf8` names.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "fail" => Some(Self::Fail),
            "replace" => Some(Self::Replace),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Settings {
    grant: Grant,
    invalid_utf8: InvalidUtf8,
}

static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

/// Fix the grant and the text setting every later evaluation reads.
pub fn set_policy(grant: Grant, invalid_utf8: InvalidUtf8) {
    *SETTINGS.lock().unwrap_or_else(PoisonError::into_inner) = Some(Settings {
        grant,
        invalid_utf8,
    });
}

fn settings() -> Settings {
    SETTINGS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .unwrap_or_default()
}

/// What a child's text that is not UTF-8 becomes.
pub(crate) fn invalid_utf8() -> InvalidUtf8 {
    settings().invalid_utf8
}

/// Whether a batch file is named: run by `cmd.exe`, so a shell.
fn batch(program: &str) -> bool {
    let lower = program.to_ascii_lowercase();
    lower.ends_with(".bat") || lower.ends_with(".cmd")
}

/// Whether the host lets this run start `spec`'s program.
///
/// # Errors
/// Names the program when a document is rendered or the grant does not
/// list it.
pub(crate) fn permitted(spec: &CommandSpec) -> Result<(), String> {
    if matches!(crate::io::policy(), Policy::Document { .. }) {
        return Err(format!(
            "command `{}`: a document starts no program",
            spec.program
        ));
    }
    match settings().grant {
        Grant::All => Ok(()),
        Grant::Only(programs) => {
            let listed = |name: &str| programs.iter().any(|program| program == name);
            if !listed(&spec.program) {
                return Err(format!(
                    "command `{}`: the program is not in the project's `commands` grant",
                    spec.program
                ));
            }
            if batch(&spec.program) && !listed("cmd") {
                return Err(format!(
                    "command `{}`: a batch file is run by `cmd`, which the project's `commands` \
                     grant does not list",
                    spec.program
                ));
            }
            Ok(())
        }
    }
}

/// A document starts no program: `command` is refused where it is written.
fn refuse_in_documents(
    _: &mut dyn CheckCx,
    _: &CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    if matches!(crate::io::policy(), Policy::Document { .. }) {
        return Err(Diagnostic::typing(span, "a document starts no program"));
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

fn eval_command(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let program = text(input, "a program", &span)?;
    if program.is_empty() {
        return Err(Diagnostic::runtime(span, "a program is named, not empty"));
    }
    let unnamed: Vec<&TypedArg> = arguments
        .iter()
        .filter(|argument| argument.name.is_none())
        .collect();
    let args = match super::named(arguments, "args").or_else(|| unnamed.first().copied()) {
        Some(argument) => {
            let value = cx.evaluate(&argument.value)?;
            let elements = value.elements().ok_or_else(|| {
                Diagnostic::runtime(span.clone(), "the arguments are a list of strings")
            })?;
            elements
                .into_iter()
                .map(|element| text(element, "an argument", &span))
                .collect::<Result<Vec<_>, _>>()?
        }
        None => Vec::new(),
    };
    let dir = match super::named(arguments, "dir") {
        Some(argument) => {
            let dir = text(cx.evaluate(&argument.value)?, "a directory", &span)?;
            (!dir.is_empty()).then(|| crate::io::resolved(Path::new(&dir)))
        }
        None => None,
    };
    let env = match super::named(arguments, "env") {
        Some(argument) => variables(&cx.evaluate(&argument.value)?, &span)?,
        None => BTreeMap::new(),
    };
    let timeout = match super::named(arguments, "timeout_ms") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Int(0) => None,
            Value::Int(count) => Some(u64::try_from(count).map(Duration::from_millis).map_err(
                |_| {
                    Diagnostic::runtime(
                        span.clone(),
                        "`timeout_ms` is a count of milliseconds, above 0, or 0 for none",
                    )
                },
            )?),
            other => {
                return Err(Diagnostic::runtime(
                    span,
                    format!("`timeout_ms` is an Int, not {}", other.type_of()),
                ));
            }
        },
        None => None,
    };
    let executable = if program.contains('/') || program.contains(std::path::MAIN_SEPARATOR) {
        crate::io::resolved(Path::new(&program))
    } else {
        PathBuf::from(&program)
    };
    Ok(Value::Endpoint(Arc::new(Endpoint::Command(Box::new(
        CommandSpec {
            program,
            executable,
            args,
            dir,
            env,
            timeout,
        },
    )))))
}

/// The variables `env` adds: a map of names to text.
fn variables(value: &Value, span: &Range<usize>) -> Result<BTreeMap<String, String>, Diagnostic> {
    let refused = || Diagnostic::runtime(span.clone(), "`env` is a map of names to text values");
    let Value::Data(data) = value else {
        return Err(refused());
    };
    let Data::Map(entries) = data.as_ref() else {
        return Err(refused());
    };
    entries
        .iter()
        .map(|(name, value)| match value {
            Data::Str(text) => Ok((name.to_string(), text.to_string())),
            _ => Err(refused()),
        })
        .collect()
}

/// A record of a standard type, its fields in declaration order.
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

fn string(text: String) -> Value {
    Value::Str(Arc::from(text))
}

/// `CommandFailure` for `failure` of the command `endpoint`.
fn failure_record(endpoint: &Arc<Endpoint>, failure: crate::process::Failure) -> Value {
    let status = match failure.status {
        Some(status) => Value::Present(Arc::new(Value::Int(i64::from(status)))),
        None => Value::Absent(TypeRef::INT),
    };
    let signal = match failure.signal {
        Some(number) => Value::Present(Arc::new(record(
            TypeRef::SIGNAL,
            vec![
                ("name", string(crate::process::signal_name(number))),
                ("number", Value::Int(i64::from(number))),
            ],
        ))),
        None => Value::Absent(TypeRef::SIGNAL),
    };
    record(
        TypeRef::COMMAND_FAILURE,
        vec![
            ("command", Value::Endpoint(Arc::clone(endpoint))),
            ("reason", string(failure.reason.to_owned())),
            ("status", status),
            ("signal", signal),
            (
                "stdout",
                string(String::from_utf8_lossy(&failure.stdout).into_owned()),
            ),
            (
                "stderr",
                string(String::from_utf8_lossy(&failure.stderr).into_owned()),
            ),
        ],
    )
}

fn eval_run(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let Value::Endpoint(endpoint) = input else {
        return Err(Diagnostic::runtime(span, "`run` takes a Command"));
    };
    let Endpoint::Command(spec) = endpoint.as_ref() else {
        return Err(Diagnostic::runtime(
            span,
            format!("`run` takes a Command, not {}", endpoint.describe()),
        ));
    };
    let written = match super::named_or_first(arguments, "input") {
        Some(argument) => text(cx.evaluate(&argument.value)?, "the input", &span)?,
        None => String::new(),
    };
    match crate::process::run(spec, written.as_bytes()) {
        Ok(ran) => {
            let (Some(stdout), Some(stderr)) = (
                crate::process::text(ran.stdout.clone()),
                crate::process::text(ran.stderr.clone()),
            ) else {
                let failure = crate::process::Failure {
                    reason: "invalid-text",
                    stdout: ran.stdout,
                    stderr: ran.stderr,
                    ..crate::process::Failure::default()
                };
                return Ok(Value::Err(
                    Arc::new(failure_record(&endpoint, failure)),
                    TypeRef::OUTPUT,
                ));
            };
            let output = record(
                TypeRef::OUTPUT,
                vec![("stdout", string(stdout)), ("stderr", string(stderr))],
            );
            Ok(Value::Ok(Arc::new(output), TypeRef::COMMAND_FAILURE))
        }
        Err(crate::process::Problem::Failed(failure)) => Ok(Value::Err(
            Arc::new(failure_record(&endpoint, failure)),
            TypeRef::OUTPUT,
        )),
        Err(problem @ crate::process::Problem::Refused(_)) => {
            Err(Diagnostic::io_failure(span, problem.message(spec)))
        }
    }
}
