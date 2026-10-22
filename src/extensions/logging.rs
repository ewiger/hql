//! The `logging` extension: structured events a `Logger` writes to one
//! endpoint, by the rules of `sink`.
//!
//! A logger adds no output of its own. It names a producer, holds a
//! threshold, and writes each event as `sink` would: one document per event
//! to a file, a standard stream or the network, or the `LogEvent` value itself
//! to a channel. See [HQL-0021](../../doc/proposals/HQL-0021/README.md).

use super::Signature;
use super::spec::{
    ArgumentKind as A, ArgumentSpec, CheckedCall, OperationSpec, Output as O, TypeParameter,
    TypePattern as P,
};
use super::{CheckCx, EvalCx, Operation, Purity};
use crate::data::Data;
use crate::diagnostics::Diagnostic;
use crate::execution::{Arg as TypedArg, Kind as TypedKind};
use crate::io::{self, Destination, Endpoint, Policy};
use crate::types::{TypeConstructor, TypeRef, Value};
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, Mutex};

/// How severe an event is, least to most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Finer than a program usually wants.
    Trace,
    /// What a developer looks at.
    Debug,
    /// What a run states.
    Info,
    /// Something that may be wrong.
    Warn,
    /// Something that is wrong.
    Error,
}

impl Level {
    /// Every level, least severe first.
    pub const ALL: [Self; 5] = [
        Self::Trace,
        Self::Debug,
        Self::Info,
        Self::Warn,
        Self::Error,
    ];

    /// The level's type.
    #[must_use]
    pub fn type_of(self) -> TypeRef {
        match self {
            Self::Trace => TypeRef::TRACE,
            Self::Debug => TypeRef::DEBUG,
            Self::Info => TypeRef::INFO,
            Self::Warn => TypeRef::WARN,
            Self::Error => TypeRef::ERROR,
        }
    }

    /// The level a type names, `Info` for `Info`.
    #[must_use]
    pub fn of_type(ty: &TypeRef) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.type_of() == *ty)
    }

    /// The level a lowercase word names, as `--log-level` writes it.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.name().eq_ignore_ascii_case(word))
    }

    /// The level's name, `Info`.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.type_of().constructor.0
    }

    /// The operation that logs at this level.
    fn operation(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// One structured event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// How severe it is.
    pub level: Level,
    /// The producer's name.
    pub logger: String,
    /// What happened.
    pub message: String,
    /// Structured context, an empty map when none was given.
    pub data: Data,
}

impl Event {
    /// The event as one JSON document.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "level": self.level.name(),
            "logger": self.logger,
            "message": self.message,
            "data": self.data.to_json(),
        })
    }
}

impl fmt::Display for Event {
    /// One line: the level in capitals, the logger, the message, then the
    /// data unless it is empty.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.level.name().to_uppercase(),
            self.logger,
            self.message
        )?;
        let empty = matches!(&self.data, Data::Map(entries) if entries.is_empty())
            || matches!(self.data, Data::Empty);
        if !empty {
            write!(
                f,
                " {}",
                Value::Data(Arc::new(self.data.clone())).to_literal()
            )?;
        }
        Ok(())
    }
}

/// A producer of events, and where it writes them.
pub struct Logger {
    /// Its name, dotted by convention.
    pub name: String,
    /// The least severe level it writes.
    pub level: Level,
    to: Endpoint,
    format: Option<TypeConstructor>,
    /// The destination, opened by the first event, so a file is replaced once
    /// however many events are written to it.
    out: Mutex<Option<(Destination, bool)>>,
}

impl fmt::Debug for Logger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Logger")
            .field("name", &self.name)
            .field("level", &self.level)
            .field("to", &self.to)
            .finish_non_exhaustive()
    }
}

impl Logger {
    /// The literal-like name a logger prints as.
    #[must_use]
    pub fn describe(&self) -> String {
        format!("Logger({}, name = {:?})", self.to, self.name)
    }

    /// Write one event, when its level is at least the threshold in force.
    fn write(&self, event: Event) -> Result<(), String> {
        let threshold =
            io::log_level().map_or(self.level, |word| Level::parse(&word).unwrap_or(self.level));
        if event.level < threshold {
            return Ok(());
        }
        let value = Value::Log(Arc::new(event));
        if let Endpoint::Channel(name, _) = &self.to {
            return io::push(name, value);
        }
        let format = self.format.unwrap_or(TypeConstructor("Text"));
        let mut out = self
            .out
            .lock()
            .map_err(|_| format!("the logger {:?} was poisoned", self.name))?;
        if out.is_none() {
            *out = Some(Destination::open(&self.to)?);
        }
        let Some((destination, held)) = out.as_mut() else {
            return Ok(());
        };
        let text = super::format::piece(&value, format, !*held)?;
        *held = true;
        destination.line(&text)
    }
}

const LEVEL_PARAMETER: TypeParameter = TypeParameter {
    name: "L",
    bound: Some(TypeConstructor("LogLevel")),
};

/// The five operations that log, one per level.
macro_rules! levels {
    ($($name:literal => $eval:ident, $level:expr, $summary:literal),* $(,)?) => {
        [$(Operation {
            name: $name,
            spec: Signature::Native(OperationSpec {
                parameters: &[],
                input: P::Type(TypeConstructor("Logger")),
                arguments: &[
                    ArgumentSpec::value("message", P::Type(TypeConstructor("String"))),
                    ArgumentSpec {
                        required: false,
                        ..ArgumentSpec::value("data", P::Type(TypeConstructor("Data")))
                    },
                ],
                output: O::Type(P::Type(TypeConstructor("Unit"))),
            }),
            summary: $summary,
            purity: Purity::Effectful,
            check: None,
            eval: $eval,
        }),*]
    };
}

/// Every operation the `logging` extension provides.
pub(crate) static STEPS: &[Operation] = &{
    let [trace, debug, info, warn, error] = levels! {
        "trace" => eval_trace, Level::Trace, "Write a Trace event to the logger's endpoint, when its threshold allows.",
        "debug" => eval_debug, Level::Debug, "Write a Debug event to the logger's endpoint, when its threshold allows.",
        "info" => eval_info, Level::Info, "Write an Info event to the logger's endpoint, when its threshold allows.",
        "warn" => eval_warn, Level::Warn, "Write a Warn event to the logger's endpoint, when its threshold allows.",
        "error" => eval_error, Level::Error, "Write an Error event to the logger's endpoint, when its threshold allows.",
    };
    [
        Operation {
            name: "Logger",
            spec: Signature::Native(OperationSpec {
                parameters: &[
                    TypeParameter {
                        name: "F",
                        bound: Some(crate::types::builtin::FORMAT),
                    },
                    LEVEL_PARAMETER,
                ],
                input: P::Type(TypeConstructor("Writable")),
                arguments: &[
                    ArgumentSpec {
                        name: "format",
                        kind: A::TypeName("F"),
                        required: false,
                        positional: true,
                        named: true,
                        variadic: false,
                    },
                    ArgumentSpec {
                        positional: false,
                        ..ArgumentSpec::value("name", P::Type(TypeConstructor("String")))
                    },
                    ArgumentSpec {
                        name: "level",
                        kind: A::TypeName("L"),
                        required: false,
                        positional: false,
                        named: true,
                        variadic: false,
                    },
                ],
                output: O::Type(P::Type(TypeConstructor("Logger"))),
            }),
            summary: "A logger that writes LogEvents to an endpoint in a format, or to a \
                      Channel<LogEvent> as values; `level` is the least severe it writes.",
            purity: Purity::Pure,
            check: Some(check_logger),
            eval: eval_logger,
        },
        trace,
        debug,
        info,
        warn,
        error,
    ]
};

fn check_logger(
    cx: &mut dyn CheckCx,
    call: &CheckedCall<'_>,
    span: Range<usize>,
) -> Result<(), Diagnostic> {
    // A logger writes what `sink` would, so a host checks it as one.
    if call.input.constructor == crate::types::builtin::CHANNEL {
        if call.bindings.contains_key("F") {
            return Err(Diagnostic::typing(
                span,
                "a channel holds values, not bytes, so a logger on one takes no format",
            ));
        }
        if call.input.args.first() != Some(&TypeRef::LOG_EVENT) {
            return Err(Diagnostic::typing(
                span,
                format!(
                    "a logger writes LogEvents, and {} holds other values",
                    call.input
                ),
            ));
        }
        return Ok(());
    }
    let _ = cx;
    if matches!(io::policy(), Policy::Document { .. }) {
        return Err(Diagnostic::typing(
            span,
            "a document writes nothing outside itself; log to `cell.logs`",
        ));
    }
    super::io::checked_format(call, call.input_expr, "Logger")
        .map(drop)
        .map_err(|message| Diagnostic::typing(span, message))
}

fn eval_logger(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let Value::Endpoint(to) = input else {
        return Err(Diagnostic::runtime(span, "a logger writes to an endpoint"));
    };
    let name = match super::named(arguments, "name") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Str(name) => name.to_string(),
            _ => return Err(Diagnostic::runtime(span, "a logger's name is a String")),
        },
        None => return Err(Diagnostic::runtime(span, "a logger needs a `name`")),
    };
    let level = super::named(arguments, "level")
        .and_then(|argument| Level::of_type(&argument.value.ty))
        .unwrap_or(Level::Info);
    let format = arguments
        .iter()
        .find(|argument| {
            argument.name.as_deref() != Some("level")
                && matches!(argument.value.kind, TypedKind::Type)
        })
        .map(|argument| argument.value.ty.constructor)
        .or_else(|| to.path().and_then(io::format_of_extension));
    Ok(Value::Logger(Arc::new(Logger {
        name,
        level,
        to: to.as_ref().clone(),
        format,
        out: Mutex::new(None),
    })))
}

/// Write an event at `level` through the logger the input is.
fn log_at(
    level: Level,
    cx: &mut dyn EvalCx,
    input: &Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let Value::Logger(logger) = input else {
        return Err(Diagnostic::runtime(
            span,
            format!("`{}` writes through a Logger", level.operation()),
        ));
    };
    let message = match super::named_or_first(arguments, "message") {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Str(text) => text.to_string(),
            other => other.to_string(),
        },
        None => String::new(),
    };
    let data = match super::named(arguments, "data").or_else(|| {
        arguments
            .iter()
            .filter(|argument| argument.name.is_none())
            .nth(1)
    }) {
        Some(argument) => match cx.evaluate(&argument.value)? {
            Value::Data(data) => data.as_ref().clone(),
            other => crate::data::Data::Str(other.to_string()),
        },
        None => Data::map(),
    };
    logger
        .write(Event {
            level,
            logger: logger.name.clone(),
            message,
            data,
        })
        .map_err(|message| Diagnostic::runtime(span, message))?;
    Ok(Value::Unit)
}

macro_rules! evaluations {
    ($($eval:ident => $level:expr),* $(,)?) => {
        $(fn $eval(
            cx: &mut dyn EvalCx,
            input: Value,
            arguments: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value, Diagnostic> {
            log_at($level, cx, &input, arguments, span)
        })*
    };
}

evaluations! {
    eval_trace => Level::Trace,
    eval_debug => Level::Debug,
    eval_info => Level::Info,
    eval_warn => Level::Warn,
    eval_error => Level::Error,
}
