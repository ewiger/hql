//! The `string` module: text functions, `str` and `fmt`.
//!
//! Every function here is a receiver function of `String` in `std/string.hql`
//! as well as a step: `s.trim()`, `string.trim(s)` and `s | trim` are one
//! declaration. `str` and `fmt` take any value and are the prelude's way to
//! text that is not a presentation format.

use super::declared;
use super::{EvalCx, Operation};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::types::{TypeRef, Value};
use std::ops::Range;
use std::sync::Arc;

/// Every step the `string` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    declared("str", "The text a value prints as.", eval_str),
    declared(
        "fmt",
        "The template with each `{}` replaced by the text of the next argument.",
        eval_fmt,
    ),
    declared("length", "How many characters the text has.", eval_length),
    declared("upper", "The text in upper case.", eval_upper),
    declared("lower", "The text in lower case.", eval_lower),
    declared(
        "trim",
        "The text without leading and trailing whitespace.",
        eval_trim,
    ),
    declared(
        "trim_start",
        "The text without leading whitespace.",
        eval_trim_start,
    ),
    declared(
        "trim_end",
        "The text without trailing whitespace.",
        eval_trim_end,
    ),
    declared(
        "capitalize",
        "The text with its first character in upper case.",
        eval_capitalize,
    ),
    declared(
        "split",
        "The pieces between occurrences of a separator, in order; with a limit of 1 or more, at most that many, the last holding the rest.",
        eval_split,
    ),
    declared(
        "join",
        "The texts in order, with the separator between each two.",
        eval_join,
    ),
    declared(
        "replace_all",
        "The text with every occurrence of one part replaced by another.",
        eval_replace_all,
    ),
    declared(
        "replace_first",
        "The text with the first occurrence of one part replaced by another.",
        eval_replace_first,
    ),
    declared(
        "starts_with",
        "Whether the text begins with a part.",
        eval_starts_with,
    ),
    declared(
        "ends_with",
        "Whether the text ends with a part.",
        eval_ends_with,
    ),
    declared(
        "contains",
        "Whether a part occurs in the text.",
        eval_contains,
    ),
    declared(
        "index_of",
        "The position of the first occurrence of a part, counting characters from 0.",
        eval_index_of,
    ),
    declared(
        "last_index_of",
        "The position of the last occurrence of a part, counting characters from 0.",
        eval_last_index_of,
    ),
    declared(
        "occurrences",
        "How many non-overlapping times a part occurs in the text; 0 for an empty part.",
        eval_occurrences,
    ),
    declared(
        "strip_prefix",
        "The text without a part it begins with, or absence when it does not begin with it.",
        eval_strip_prefix,
    ),
    declared(
        "strip_suffix",
        "The text without a part it ends with, or absence when it does not end with it.",
        eval_strip_suffix,
    ),
    declared(
        "trim_prefix",
        "The text without a part it begins with, or the text as it is.",
        eval_trim_prefix,
    ),
    declared(
        "trim_suffix",
        "The text without a part it ends with, or the text as it is.",
        eval_trim_suffix,
    ),
    declared(
        "equals_ignore_case",
        "Whether the two texts are equal in lower case.",
        eval_equals_ignore_case,
    ),
    declared(
        "chars",
        "Each character as its own text, in order.",
        eval_chars,
    ),
    declared(
        "lines",
        "The lines of the text, without their line breaks.",
        eval_lines,
    ),
    declared(
        "words",
        "The whitespace-separated words of the text.",
        eval_words,
    ),
    declared(
        "reverse",
        "The characters in the opposite order.",
        eval_reverse,
    ),
    declared(
        "repeat",
        "The text written a number of times in a row.",
        eval_repeat,
    ),
    declared(
        "pad_left",
        "The text preceded by a fill until it is a width long.",
        eval_pad_left,
    ),
    declared(
        "pad_right",
        "The text followed by a fill until it is a width long.",
        eval_pad_right,
    ),
    declared(
        "center",
        "The text between fills until it is a width long; an odd fill character goes after it.",
        eval_center,
    ),
    declared(
        "truncate",
        "At most the first width characters of the text.",
        eval_truncate,
    ),
    declared(
        "slice",
        "The characters from one position up to another, counting from 0.",
        eval_slice,
    ),
    declared(
        "char_at",
        "The character at a position, counting from 0, or absence past the end.",
        eval_char_at,
    ),
    declared(
        "to_int",
        "The integer the text writes, or absence when it writes none.",
        eval_to_int,
    ),
    declared(
        "to_float",
        "The number the text writes, or absence when it writes none.",
        eval_to_float,
    ),
    declared(
        "to_bool",
        "The truth value the text writes as `true` or `false`, or absence when it writes neither.",
        eval_to_bool,
    ),
    declared(
        "is_empty",
        "Whether the text has no characters.",
        eval_is_empty,
    ),
    declared(
        "is_blank",
        "Whether the text has no characters other than whitespace.",
        eval_is_blank,
    ),
    declared(
        "ord",
        "The Unicode code point of a one-character text, or absence for any other text.",
        eval_ord,
    ),
    declared(
        "chr",
        "The one-character text of a Unicode code point, or absence for an Int that is none.",
        eval_chr,
    ),
    declared(
        "bytes",
        "The UTF-8 encoding of the text, one Int per byte.",
        eval_bytes,
    ),
];

fn text(value: &str) -> Value {
    Value::Str(Arc::from(value))
}

/// The character position of a byte offset into a text.
fn position(s: &str, byte: usize) -> Value {
    Value::Int(i64::try_from(s[..byte].chars().count()).unwrap_or(i64::MAX))
}

/// A text when there is one, absence otherwise.
fn maybe_text(value: Option<&str>) -> Value {
    value.map_or(Value::Absent(TypeRef::STR), |value| {
        Value::Present(Arc::new(text(value)))
    })
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
    eval_is_blank => "is_blank", |s| Value::Bool(s.trim().is_empty());
    eval_to_bool => "to_bool", |s| match s.trim() {
        "true" => Value::Present(Arc::new(Value::Bool(true))),
        "false" => Value::Present(Arc::new(Value::Bool(false))),
        _ => Value::Absent(TypeRef::BOOL),
    };
    eval_ord => "ord", |s| {
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Value::Present(Arc::new(Value::Int(i64::from(u32::from(c))))),
            _ => Value::Absent(TypeRef::INT),
        }
    };
    eval_bytes => "bytes", |s| Value::list(
        s.bytes().map(|byte| Value::Int(i64::from(byte))).collect(),
        TypeRef::INT,
    );
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
    eval_starts_with => "starts_with", |s, part| Value::Bool(s.starts_with(part.as_str()));
    eval_ends_with => "ends_with", |s, part| Value::Bool(s.ends_with(part.as_str()));
    eval_contains => "contains", |s, part| Value::Bool(s.contains(part.as_str()));
    eval_index_of => "index_of", |s, part| match s.find(part.as_str()) {
        Some(byte) => Value::Present(Arc::new(position(&s, byte))),
        None => Value::Absent(TypeRef::INT),
    };
    eval_last_index_of => "last_index_of", |s, part| match s.rfind(part.as_str()) {
        Some(byte) => Value::Present(Arc::new(position(&s, byte))),
        None => Value::Absent(TypeRef::INT),
    };
    eval_occurrences => "occurrences", |s, part| Value::Int(if part.is_empty() {
        0
    } else {
        i64::try_from(s.matches(part.as_str()).count()).unwrap_or(i64::MAX)
    });
    eval_strip_prefix => "strip_prefix", |s, part| maybe_text(s.strip_prefix(part.as_str()));
    eval_strip_suffix => "strip_suffix", |s, part| maybe_text(s.strip_suffix(part.as_str()));
    eval_trim_prefix => "trim_prefix", |s, part| text(s.strip_prefix(part.as_str()).unwrap_or(&s));
    eval_trim_suffix => "trim_suffix", |s, part| text(s.strip_suffix(part.as_str()).unwrap_or(&s));
    eval_equals_ignore_case => "equals_ignore_case", |s, other| {
        Value::Bool(s.to_lowercase() == other.to_lowercase())
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

/// The text with the first occurrence, or every occurrence, of one part
/// replaced by another.
fn replaced(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
    name: &str,
    every: bool,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, name, &span)?;
    let from = argument_text(cx, arguments, 0, name, &span)?;
    let to = argument_text(cx, arguments, 1, name, &span)?;
    Ok(text(&if every {
        s.replace(from.as_str(), &to)
    } else {
        s.replacen(from.as_str(), &to, 1)
    }))
}

fn eval_replace_all(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    replaced(cx, input, arguments, span, "replace_all", true)
}

fn eval_replace_first(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    replaced(cx, input, arguments, span, "replace_first", false)
}

/// The pieces between separators; a limit of 1 or more keeps at most that
/// many, and an empty separator splits between characters.
fn eval_split(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, "split", &span)?;
    let separator = argument_text(cx, arguments, 0, "split", &span)?;
    let limit = match super::named(arguments, "limit").or_else(|| arguments.get(1)) {
        Some(limit) => match cx.evaluate(&limit.value)? {
            Value::Int(limit) => usize::try_from(limit).unwrap_or(0),
            other => {
                return Err(Diagnostic::runtime(
                    span,
                    format!("`split` needs an Int limit, not {}", other.type_of()),
                ));
            }
        },
        None => 0,
    };
    let pieces: Vec<String> = match (separator.is_empty(), limit) {
        (true, 0) => s.chars().map(String::from).collect(),
        (true, limit) => {
            let mut pieces: Vec<String> = s.chars().take(limit - 1).map(String::from).collect();
            let rest: String = s.chars().skip(limit - 1).collect();
            if !rest.is_empty() {
                pieces.push(rest);
            }
            pieces
        }
        (false, 0) => s.split(separator.as_str()).map(str::to_owned).collect(),
        (false, limit) => s
            .splitn(limit, separator.as_str())
            .map(str::to_owned)
            .collect(),
    };
    Ok(texts(pieces))
}

fn eval_truncate(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, "truncate", &span)?;
    let width = usize::try_from(argument_int(cx, arguments, 0, "truncate", &span)?).unwrap_or(0);
    Ok(text(&s.chars().take(width).collect::<String>()))
}

fn eval_center(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let s = input_text(&input, "center", &span)?;
    let width = usize::try_from(argument_int(cx, arguments, 0, "center", &span)?).unwrap_or(0);
    let fill = argument_text(cx, arguments, 1, "center", &span)?;
    let length = s.chars().count();
    if fill.is_empty() || length >= width {
        return Ok(text(&s));
    }
    let before = (width - length) / 2;
    let after = width - length - before;
    let before: String = fill.chars().cycle().take(before).collect();
    let after: String = fill.chars().cycle().take(after).collect();
    Ok(text(&(before + &s + &after)))
}

/// The one-character text of an Int that is a Unicode scalar value.
fn eval_chr(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value, Diagnostic> {
    let code = match input {
        Value::Int(code) => code,
        other => {
            return Err(Diagnostic::runtime(
                span,
                format!("`chr` needs an Int, not {}", other.type_of()),
            ));
        }
    };
    Ok(maybe_text(
        u32::try_from(code)
            .ok()
            .and_then(char::from_u32)
            .map(String::from)
            .as_deref(),
    ))
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
    // `s.slice(start, end)` is `s[start:end]`: clamped, and a negative
    // bound counts from the end.
    let s = input_text(&input, "slice", &span)?;
    let start = argument_int(cx, arguments, 0, "slice", &span)?;
    let end = argument_int(cx, arguments, 1, "slice", &span)?;
    crate::evaluation::slice_of(text(&s), Some(start), Some(end), &span)
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
            step.spec()
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

    fn eval(program: &str) -> String {
        crate::eval(program)
            .unwrap_or_else(|error| panic!("{program}: {error:?}"))
            .to_string()
    }

    #[test]
    fn search_counts_and_finds_the_last_occurrence() {
        assert_eq!(eval(r#""a.b.c".last_index_of(".").unwrap_or(-1)"#), "3");
        assert_eq!(eval(r#""é.é".last_index_of("é").unwrap_or(-1)"#), "2");
        assert_eq!(eval(r#""banana".occurrences("an")"#), "2");
        assert_eq!(eval(r#""aaaa".occurrences("aa")"#), "2");
        assert_eq!(eval(r#""abc".occurrences("")"#), "0");
    }

    #[test]
    fn strip_is_optional_and_trim_keeps_the_text() {
        assert_eq!(eval(r#""v1.2".strip_prefix("v").unwrap_or("-")"#), "1.2");
        assert_eq!(eval(r#""v1.2".strip_prefix("x").unwrap_or("-")"#), "-");
        assert_eq!(eval(r#""a.md".strip_suffix(".md").unwrap_or("-")"#), "a");
        assert_eq!(eval(r#""a.md".trim_prefix("x")"#), "a.md");
        assert_eq!(eval(r#""a.md".trim_suffix(".md")"#), "a");
    }

    #[test]
    fn a_split_limit_keeps_the_rest_in_the_last_piece() {
        assert_eq!(eval(r#""a,b,c".split(",").join("|")"#), "a|b|c");
        assert_eq!(eval(r#""a,b,c".split(",", 2).join("|")"#), "a|b,c");
        assert_eq!(eval(r#""a,b,c".split(",", limit = 2).join("|")"#), "a|b,c");
        assert_eq!(eval(r#""a,b,c".split(",", 9).join("|")"#), "a|b|c");
        assert_eq!(eval(r#""a,b,c".split(",", 0).join("|")"#), "a|b|c");
        assert_eq!(eval(r#""abc".split("", 2).join("|")"#), "a|bc");
    }

    #[test]
    fn predicates_and_conversions() {
        assert_eq!(eval(r#"" \t".is_blank()"#), "true");
        assert_eq!(eval(r#""x ".is_blank()"#), "false");
        assert_eq!(eval(r#""Straße".equals_ignore_case("STRASSE")"#), "false");
        assert_eq!(eval(r#""HQL".equals_ignore_case("hql")"#), "true");
        assert_eq!(eval(r#"" true ".to_bool().unwrap_or(false)"#), "true");
        assert_eq!(eval(r#""yes".to_bool().is_some()"#), "false");
    }

    #[test]
    fn characters_and_codes_round_trip() {
        assert_eq!(eval(r#""λ".ord().unwrap_or(0)"#), "955");
        assert_eq!(eval(r#""ab".ord().is_some()"#), "false");
        assert_eq!(eval(r#"chr(955).unwrap_or("-")"#), "λ");
        assert_eq!(eval(r#"chr(55296).unwrap_or("-")"#), "-");
        assert_eq!(eval(r#"chr(-1).unwrap_or("-")"#), "-");
        assert_eq!(eval(r#""é".bytes()"#), "[195, 169]");
    }

    #[test]
    fn truncate_and_center_lay_out_by_characters() {
        assert_eq!(eval(r#""Hello".truncate(2)"#), "He");
        assert_eq!(eval(r#""Hi".truncate(9)"#), "Hi");
        assert_eq!(eval(r#""ab".center(7, "*")"#), "**ab***");
        assert_eq!(eval(r#""abc".center(2, "*")"#), "abc");
    }

    #[test]
    fn replace_says_first_or_all() {
        assert_eq!(eval(r#""aXbXc".replace_first("X", "-")"#), "a-bXc");
        assert_eq!(eval(r#""aXbXc".replace_all("X", "-")"#), "a-b-c");
    }
}
