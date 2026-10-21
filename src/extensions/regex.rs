//! The `regex` module: matching, finding, replacing, capturing and splitting
//! text by a pattern. Imported where it is used, since a pattern language is
//! a domain of its own.
//!
//! A pattern is a `String`; one that does not parse is an evaluation error
//! at the call, naming what the pattern engine refused.

use super::spec::{ArgumentSpec, OperationSpec, Output as O, TypePattern as P};
use super::{EvalCx, Operation, Purity};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::collections::LIST;
use crate::types::{TypeConstructor, TypeRef, Value};
use std::ops::Range;
use std::sync::Arc;

const STRING: TypeConstructor = TypeConstructor("String");
const BOOL: TypeConstructor = TypeConstructor("Bool");
const OPTION: TypeConstructor = TypeConstructor("Option");

const PATTERN: ArgumentSpec = ArgumentSpec {
    named: false,
    ..ArgumentSpec::value("pattern", P::Type(STRING))
};

const fn over_text(
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

/// Every step the `regex` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    over_text(
        "is_match",
        "Whether the pattern matches somewhere in the text.",
        &[PATTERN],
        P::Type(BOOL),
        eval_is_match,
    ),
    over_text(
        "find",
        "The first match of the pattern, or absence.",
        &[PATTERN],
        P::Apply(OPTION, &[P::Type(STRING)]),
        eval_find,
    ),
    over_text(
        "find_all",
        "Every non-overlapping match of the pattern, in order.",
        &[PATTERN],
        P::Apply(LIST, &[P::Type(STRING)]),
        eval_find_all,
    ),
    over_text(
        "captures",
        "The first match and its groups, whole match first, or absence; a group that did not take part is empty.",
        &[PATTERN],
        P::Apply(OPTION, &[P::Apply(LIST, &[P::Type(STRING)])]),
        eval_captures,
    ),
    over_text(
        "replace",
        "The text with every match replaced; `$1` in the replacement is the first group.",
        &[
            PATTERN,
            ArgumentSpec {
                named: false,
                ..ArgumentSpec::value("replacement", P::Type(STRING))
            },
        ],
        P::Type(STRING),
        eval_replace,
    ),
    over_text(
        "split",
        "The pieces between matches of the pattern, in order.",
        &[PATTERN],
        P::Apply(LIST, &[P::Type(STRING)]),
        eval_split,
    ),
];

fn text(value: &str) -> Value {
    Value::Str(Arc::from(value))
}

fn texts<'a>(values: impl IntoIterator<Item = &'a str>) -> Value {
    Value::list(values.into_iter().map(text).collect(), TypeRef::STR)
}

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

/// The pattern an argument holds, compiled.
fn pattern(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    index: usize,
    name: &str,
    span: &Range<usize>,
) -> Result<regex::Regex, Diagnostic> {
    let argument = arguments
        .get(index)
        .ok_or_else(|| Diagnostic::runtime(span.clone(), format!("`{name}` needs a pattern")))?;
    let value = cx.evaluate(&argument.value)?;
    let written = input_text(&value, name, &argument.value.span)?;
    regex::Regex::new(&written).map_err(|error| {
        Diagnostic::runtime(
            argument.value.span.clone(),
            format!("`{name}` cannot read the pattern {written:?}: {error}"),
        )
    })
}

fn eval_is_match(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let haystack = input_text(&input, "is_match", &span)?;
    let pattern = pattern(cx, arguments, 0, "is_match", &span)?;
    Ok(Value::Bool(pattern.is_match(&haystack)))
}

fn eval_find(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let haystack = input_text(&input, "find", &span)?;
    let pattern = pattern(cx, arguments, 0, "find", &span)?;
    Ok(match pattern.find(&haystack) {
        Some(found) => Value::Present(Arc::new(text(found.as_str()))),
        None => Value::Absent(TypeRef::STR),
    })
}

fn eval_find_all(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let haystack = input_text(&input, "find_all", &span)?;
    let pattern = pattern(cx, arguments, 0, "find_all", &span)?;
    Ok(texts(
        pattern.find_iter(&haystack).map(|found| found.as_str()),
    ))
}

fn eval_captures(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let haystack = input_text(&input, "captures", &span)?;
    let pattern = pattern(cx, arguments, 0, "captures", &span)?;
    Ok(match pattern.captures(&haystack) {
        Some(captures) => Value::Present(Arc::new(texts(
            captures
                .iter()
                .map(|group| group.map_or("", |group| group.as_str())),
        ))),
        None => Value::Absent(TypeRef::list(TypeRef::STR)),
    })
}

fn eval_replace(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let haystack = input_text(&input, "replace", &span)?;
    let pattern = pattern(cx, arguments, 0, "replace", &span)?;
    let replacement = arguments
        .get(1)
        .ok_or_else(|| Diagnostic::runtime(span.clone(), "`replace` needs a replacement"))?;
    let replacement = cx.evaluate(&replacement.value)?;
    let replacement = input_text(&replacement, "replace", &span)?;
    Ok(text(&pattern.replace_all(&haystack, replacement.as_str())))
}

fn eval_split(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let haystack = input_text(&input, "split", &span)?;
    let pattern = pattern(cx, arguments, 0, "split", &span)?;
    Ok(texts(pattern.split(&haystack)))
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
    fn a_pattern_that_does_not_parse_fails_at_the_call() {
        let failed = crate::eval("import regex\nregex.is_match(\"a\", \"(\")").unwrap_err();
        assert!(
            failed.message().contains("cannot read the pattern"),
            "{}",
            failed.message()
        );
    }
}
