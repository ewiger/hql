//! The `string` module: text functions, `str` and `fmt`.
//!
//! Every function here is a receiver function of `String` in `std/string.hql`
//! as well as a step: `s.trim()`, `string.trim(s)` and `s | trim` are one
//! declaration. `str` and `fmt` take any value and are the prelude's way to
//! text that is not a presentation format.

use super::spec::{
    ArgumentKind as A, ArgumentSpec, OperationSpec, Output as O, TypeParameter, TypePattern as P,
};
use super::{EvalCx, Operation, Purity};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::collections::LIST;
use crate::types::{TypeConstructor, TypeRef, Value};
use std::ops::Range;
use std::sync::Arc;

const STRING: TypeConstructor = TypeConstructor("String");
const INT: TypeConstructor = TypeConstructor("Int");
const FLOAT: TypeConstructor = TypeConstructor("Float");
const BOOL: TypeConstructor = TypeConstructor("Bool");
const OPTION: TypeConstructor = TypeConstructor("Option");

const fn pure(
    name: &'static str,
    summary: &'static str,
    arguments: &'static [ArgumentSpec],
    output: P,
    eval: super::Eval,
) -> Operation {
    Operation {
        name,
        spec: OperationSpec {
            parameters: &[],
            input: P::Type(STRING),
            arguments,
            output: O::Type(output),
        },
        summary,
        purity: Purity::Pure,
        check: None,
        eval,
    }
}

const TEXT: ArgumentSpec = ArgumentSpec {
    named: false,
    ..ArgumentSpec::value("part", P::Type(STRING))
};

/// Every step the `string` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    Operation {
        name: "str",
        spec: OperationSpec {
            parameters: &[TypeParameter {
                name: "T",
                bound: None,
            }],
            input: P::Var("T"),
            arguments: &[],
            output: O::Type(P::Type(STRING)),
        },
        summary: "The text a value prints as.",
        purity: Purity::Pure,
        check: None,
        eval: eval_str,
    },
    Operation {
        name: "fmt",
        spec: OperationSpec {
            parameters: &[],
            input: P::Type(STRING),
            arguments: &[ArgumentSpec {
                name: "args",
                kind: A::Value(P::Any),
                required: false,
                positional: true,
                named: false,
                variadic: true,
            }],
            output: O::Type(P::Type(STRING)),
        },
        summary: "The template with each `{}` replaced by the text of the next argument.",
        purity: Purity::Pure,
        check: None,
        eval: eval_fmt,
    },
    pure(
        "length",
        "How many characters the text has.",
        &[],
        P::Type(INT),
        eval_length,
    ),
    pure(
        "upper",
        "The text in upper case.",
        &[],
        P::Type(STRING),
        eval_upper,
    ),
    pure(
        "lower",
        "The text in lower case.",
        &[],
        P::Type(STRING),
        eval_lower,
    ),
    pure(
        "trim",
        "The text without leading and trailing whitespace.",
        &[],
        P::Type(STRING),
        eval_trim,
    ),
    pure(
        "trim_start",
        "The text without leading whitespace.",
        &[],
        P::Type(STRING),
        eval_trim_start,
    ),
    pure(
        "trim_end",
        "The text without trailing whitespace.",
        &[],
        P::Type(STRING),
        eval_trim_end,
    ),
    pure(
        "capitalize",
        "The text with its first character in upper case.",
        &[],
        P::Type(STRING),
        eval_capitalize,
    ),
    pure(
        "split",
        "The pieces between occurrences of a separator, in order.",
        &[ArgumentSpec {
            named: false,
            ..ArgumentSpec::value("separator", P::Type(STRING))
        }],
        P::Apply(LIST, &[P::Type(STRING)]),
        eval_split,
    ),
    Operation {
        name: "join",
        spec: OperationSpec {
            parameters: &[],
            input: P::Collection(&P::Type(STRING)),
            arguments: &[ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("separator", P::Type(STRING))
            }],
            output: O::Type(P::Type(STRING)),
        },
        summary: "The texts in order, with the separator between each two.",
        purity: Purity::Pure,
        check: None,
        eval: eval_join,
    },
    pure(
        "replace",
        "The text with every occurrence of one part replaced by another.",
        &[
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("from", P::Type(STRING))
            },
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("to", P::Type(STRING))
            },
        ],
        P::Type(STRING),
        eval_replace,
    ),
    pure(
        "starts_with",
        "Whether the text begins with a part.",
        &[TEXT],
        P::Type(BOOL),
        eval_starts_with,
    ),
    pure(
        "ends_with",
        "Whether the text ends with a part.",
        &[TEXT],
        P::Type(BOOL),
        eval_ends_with,
    ),
    pure(
        "contains",
        "Whether a part occurs in the text.",
        &[TEXT],
        P::Type(BOOL),
        eval_contains,
    ),
    pure(
        "index_of",
        "The position of the first occurrence of a part, counting characters from 0.",
        &[TEXT],
        P::Apply(OPTION, &[P::Type(INT)]),
        eval_index_of,
    ),
    pure(
        "chars",
        "Each character as its own text, in order.",
        &[],
        P::Apply(LIST, &[P::Type(STRING)]),
        eval_chars,
    ),
    pure(
        "lines",
        "The lines of the text, without their line breaks.",
        &[],
        P::Apply(LIST, &[P::Type(STRING)]),
        eval_lines,
    ),
    pure(
        "words",
        "The whitespace-separated words of the text.",
        &[],
        P::Apply(LIST, &[P::Type(STRING)]),
        eval_words,
    ),
    pure(
        "reverse",
        "The characters in the opposite order.",
        &[],
        P::Type(STRING),
        eval_reverse,
    ),
    pure(
        "repeat",
        "The text written a number of times in a row.",
        &[ArgumentSpec {
            named: false,
            ..ArgumentSpec::value("times", P::Type(INT))
        }],
        P::Type(STRING),
        eval_repeat,
    ),
    pure(
        "pad_left",
        "The text preceded by a fill until it is a width long.",
        &[
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("width", P::Type(INT))
            },
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("fill", P::Type(STRING))
            },
        ],
        P::Type(STRING),
        eval_pad_left,
    ),
    pure(
        "pad_right",
        "The text followed by a fill until it is a width long.",
        &[
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("width", P::Type(INT))
            },
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("fill", P::Type(STRING))
            },
        ],
        P::Type(STRING),
        eval_pad_right,
    ),
    pure(
        "slice",
        "The characters from one position up to another, counting from 0.",
        &[
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("start", P::Type(INT))
            },
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("end", P::Type(INT))
            },
        ],
        P::Type(STRING),
        eval_slice,
    ),
    pure(
        "char_at",
        "The character at a position, counting from 0, or absence past the end.",
        &[ArgumentSpec {
            named: false,
            ..ArgumentSpec::value("position", P::Type(INT))
        }],
        P::Apply(OPTION, &[P::Type(STRING)]),
        eval_char_at,
    ),
    pure(
        "to_int",
        "The integer the text writes, or absence when it writes none.",
        &[],
        P::Apply(OPTION, &[P::Type(INT)]),
        eval_to_int,
    ),
    pure(
        "to_float",
        "The number the text writes, or absence when it writes none.",
        &[],
        P::Apply(OPTION, &[P::Type(FLOAT)]),
        eval_to_float,
    ),
    pure(
        "is_empty",
        "Whether the text has no characters.",
        &[],
        P::Type(BOOL),
        eval_is_empty,
    ),
];

fn text(value: &str) -> Value {
    Value::Str(Arc::from(value))
}

fn texts(values: impl IntoIterator<Item = String>) -> Value {
    Value::list(
        values.into_iter().map(|value| text(&value)).collect(),
        TypeRef::STR,
    )
}

/// The text a checked `String` input holds.
fn input_text(input: &Value, name: &str, span: &Range<usize>) -> Result<String, Diagnostic> {
    match input {
        Value::Str(text) => Ok(text.to_string()),
        Value::Formatted(formatted) => Ok(formatted.text.clone()),
        Value::Present(inner) => input_text(inner, name, span),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` needs a String, not {}", other.type_of()),
        )),
    }
}

/// The text of the argument at a position.
fn argument_text(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    index: usize,
    name: &str,
    span: &Range<usize>,
) -> Result<String, Diagnostic> {
    let argument = arguments
        .get(index)
        .ok_or_else(|| Diagnostic::runtime(span.clone(), format!("`{name}` needs an argument")))?;
    let value = cx.evaluate(&argument.value)?;
    input_text(&value, name, span)
}

/// The integer the argument at a position holds.
fn argument_int(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    index: usize,
    name: &str,
    span: &Range<usize>,
) -> Result<i64, Diagnostic> {
    let argument = arguments
        .get(index)
        .ok_or_else(|| Diagnostic::runtime(span.clone(), format!("`{name}` needs an argument")))?;
    match cx.evaluate(&argument.value)? {
        Value::Int(n) => Ok(n),
        other => Err(Diagnostic::runtime(
            span.clone(),
            format!("`{name}` needs an Int, not {}", other.type_of()),
        )),
    }
}

fn eval_str(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    _: Range<usize>,
) -> Result<Value, Diagnostic> {
    Ok(text(&input.to_string()))
}

/// `{}` placeholders filled in order; `{{` and `}}` write one brace.
fn eval_fmt(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let template = input_text(&input, "fmt", &span)?;
    let mut values = Vec::with_capacity(arguments.len());
    for argument in arguments {
        values.push(cx.evaluate(&argument.value)?);
    }
    let mut out = String::with_capacity(template.len());
    let mut next = values.iter();
    let mut rest = template.as_str();
    while let Some(position) = rest.find(['{', '}']) {
        out.push_str(&rest[..position]);
        let (brace, after) = rest[position..].split_at(1);
        if let Some(after) = after.strip_prefix(brace) {
            out.push_str(brace);
            rest = after;
            continue;
        }
        if brace == "{"
            && let Some(after) = after.strip_prefix('}')
        {
            let value = next.next().ok_or_else(|| {
                Diagnostic::runtime(
                    span.clone(),
                    format!(
                        "`fmt` has more `{{}}` placeholders than the {} argument{} given",
                        values.len(),
                        if values.len() == 1 { "" } else { "s" }
                    ),
                )
            })?;
            out.push_str(&value.to_string());
            rest = after;
            continue;
        }
        return Err(Diagnostic::runtime(
            span,
            format!("`fmt` found a stray `{brace}`; write `{brace}{brace}` for one"),
        ));
    }
    out.push_str(rest);
    if next.next().is_some() {
        return Err(Diagnostic::runtime(
            span,
            format!(
                "`fmt` was given {} arguments for fewer `{{}}` placeholders",
                values.len()
            ),
        ));
    }
    Ok(text(&out))
}

macro_rules! unary {
    ($($eval:ident => $name:literal, |$text:ident| $body:expr;)*) => {
        $(fn $eval(
            _: &mut dyn EvalCx,
            input: Value,
            _: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value, Diagnostic> {
            let $text = input_text(&input, $name, &span)?;
            Ok($body)
        })*
    };
}

unary! {
    eval_length => "length", |s| Value::Int(i64::try_from(s.chars().count()).unwrap_or(i64::MAX));
    eval_upper => "upper", |s| text(&s.to_uppercase());
    eval_lower => "lower", |s| text(&s.to_lowercase());
    eval_trim => "trim", |s| text(s.trim());
    eval_trim_start => "trim_start", |s| text(s.trim_start());
    eval_trim_end => "trim_end", |s| text(s.trim_end());
    eval_capitalize => "capitalize", |s| {
        let mut chars = s.chars();
        match chars.next() {
            Some(first) => text(&(first.to_uppercase().collect::<String>() + chars.as_str())),
            None => text(""),
        }
    };
    eval_chars => "chars", |s| texts(s.chars().map(String::from));
    eval_lines => "lines", |s| texts(s.lines().map(str::to_owned));
    eval_words => "words", |s| texts(s.split_whitespace().map(str::to_owned));
    eval_reverse => "reverse", |s| text(&s.chars().rev().collect::<String>());
    eval_is_empty => "is_empty", |s| Value::Bool(s.is_empty());
    eval_to_int => "to_int", |s| match s.trim().parse::<i64>() {
        Ok(n) => Value::Present(Arc::new(Value::Int(n))),
        Err(_) => Value::Absent(TypeRef::INT),
    };
    eval_to_float => "to_float", |s| match s.trim().parse::<f64>() {
        Ok(x) if x.is_finite() => Value::Present(Arc::new(Value::Float(x))),
        _ => Value::Absent(TypeRef::FLOAT),
    };
}

macro_rules! with_part {
    ($($eval:ident => $name:literal, |$text:ident, $part:ident| $body:expr;)*) => {
        $(fn $eval(
            cx: &mut dyn EvalCx,
            input: Value,
            arguments: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value, Diagnostic> {
            let $text = input_text(&input, $name, &span)?;
            let $part = argument_text(cx, arguments, 0, $name, &span)?;
            Ok($body)
        })*
    };
}

with_part! {
    eval_split => "split", |s, separator| {
        if separator.is_empty() {
            texts(s.chars().map(String::from))
        } else {
            texts(s.split(separator.as_str()).map(str::to_owned))
        }
    };
    eval_starts_with => "starts_with", |s, part| Value::Bool(s.starts_with(part.as_str()));
    eval_ends_with => "ends_with", |s, part| Value::Bool(s.ends_with(part.as_str()));
    eval_contains => "contains", |s, part| Value::Bool(s.contains(part.as_str()));
    eval_index_of => "index_of", |s, part| match s.find(part.as_str()) {
        Some(byte) => Value::Present(Arc::new(Value::Int(
            i64::try_from(s[..byte].chars().count()).unwrap_or(i64::MAX),
        ))),
        None => Value::Absent(TypeRef::INT),
    };
}

fn eval_join(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let separator = argument_text(cx, arguments, 0, "join", &span)?;
    let pieces = input
        .elements()
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`join` needs a collection of texts"))?
        .iter()
        .map(|piece| input_text(piece, "join", &span))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(text(&pieces.join(&separator)))
}

fn eval_replace(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, "replace", &span)?;
    let from = argument_text(cx, arguments, 0, "replace", &span)?;
    let to = argument_text(cx, arguments, 1, "replace", &span)?;
    Ok(text(&s.replace(from.as_str(), &to)))
}

fn eval_repeat(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, "repeat", &span)?;
    let times = argument_int(cx, arguments, 0, "repeat", &span)?;
    let times = usize::try_from(times).unwrap_or(0);
    Ok(text(&s.repeat(times)))
}

fn padded(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
    name: &str,
    left: bool,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, name, &span)?;
    let width = usize::try_from(argument_int(cx, arguments, 0, name, &span)?).unwrap_or(0);
    let fill = argument_text(cx, arguments, 1, name, &span)?;
    let length = s.chars().count();
    if fill.is_empty() || length >= width {
        return Ok(text(&s));
    }
    let padding: String = fill.chars().cycle().take(width - length).collect();
    Ok(text(&if left { padding + &s } else { s + &padding }))
}

fn eval_pad_left(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    padded(cx, input, arguments, span, "pad_left", true)
}

fn eval_pad_right(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    padded(cx, input, arguments, span, "pad_right", false)
}

fn eval_slice(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, "slice", &span)?;
    let start = usize::try_from(argument_int(cx, arguments, 0, "slice", &span)?).unwrap_or(0);
    let end = usize::try_from(argument_int(cx, arguments, 1, "slice", &span)?).unwrap_or(0);
    Ok(text(
        &s.chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect::<String>(),
    ))
}

fn eval_char_at(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, "char_at", &span)?;
    let position = argument_int(cx, arguments, 0, "char_at", &span)?;
    Ok(usize::try_from(position)
        .ok()
        .and_then(|position| s.chars().nth(position))
        .map_or(Value::Absent(TypeRef::STR), |c| {
            Value::Present(Arc::new(text(&c.to_string())))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_has_a_coherent_spec() {
        for step in STEPS {
            step.spec
                .validate()
                .unwrap_or_else(|error| panic!("{}: {error}", step.name));
        }
    }

    #[test]
    fn fmt_fills_placeholders_in_order_and_escapes_braces() {
        let out = crate::eval(r#"fmt("{} and {{}} and {}", 1, "two")"#).unwrap();
        assert_eq!(out.to_string(), "1 and {} and two");
        assert!(crate::eval(r#"fmt("{} {}", 1)"#).is_err());
        assert!(crate::eval(r#"fmt("{}", 1, 2)"#).is_err());
    }
}
