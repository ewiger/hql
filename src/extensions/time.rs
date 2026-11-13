//! The `time` module and the constructors of `core`'s time scalars
//! (HQL-0041 part 1): the natives behind every declaration of
//! `lib/std/time/mod.hql`, and `Date`, `DateTime`, `Instant` and
//! `Duration` of `lib/std/core/mod.hql`.
//!
//! The values and their calendar are [`crate::time`]'s; this module reads a
//! call's arguments and gives its result.

use super::{EvalCx, Operation, Purity, Shape, Signature, declared};
use crate::diagnostics::Diagnostic;
use crate::execution::Arg as TypedArg;
use crate::time::{self as t, Time, Unit};
use crate::types::{Record, TypeConstructor, TypeRef, Value};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Span, Timestamp, civil};
use std::ops::Range;
use std::sync::Arc;

// The constructors `core` declares for its time scalars, provided with
// `core`'s steps.
pub(crate) const DATE: Operation = declared("Date", eval_date);
pub(crate) const DATE_TIME: Operation = declared("DateTime", eval_datetime);
pub(crate) const INSTANT: Operation = declared("Instant", eval_instant);
pub(crate) const DURATION: Operation = declared("Duration", eval_duration);

/// An operation that reads the evaluation clock.
const fn clock(name: &'static str, eval: super::Eval) -> Operation {
    Operation {
        name,
        spec: Signature::Declared,
        purity: Purity::Effectful,
        shape: Shape::Barrier,
        check: None,
        eval,
    }
}

/// Every step the `time` module provides, in the order help prints them.
pub(crate) static STEPS: &[Operation] = &[
    declared("Zone", eval_zone),
    declared("to_date", eval_to_date),
    declared("to_datetime", eval_to_datetime),
    declared("to_instant", eval_to_instant),
    declared("to_duration", eval_to_duration),
    declared("parse_date", eval_parse_date),
    declared("parse_datetime", eval_parse_datetime),
    declared("parse_instant", eval_parse_instant),
    declared("at", eval_at),
    declared("in_zone", eval_in_zone),
    declared("offset", eval_offset),
    declared("year", eval_year),
    declared("month", eval_month),
    declared("day", eval_day),
    declared("weekday", eval_weekday),
    declared("day_of_year", eval_day_of_year),
    declared("iso_week", eval_iso_week),
    declared("iso_year", eval_iso_year),
    declared("date", eval_date_of),
    declared("hour", eval_hour),
    declared("minute", eval_minute),
    declared("second", eval_second),
    declared("nanosecond", eval_nanosecond),
    declared("unix_seconds", eval_unix_seconds),
    declared("unix_millis", eval_unix_millis),
    declared("total_seconds", eval_total_seconds),
    declared("total_millis", eval_total_millis),
    declared("plus", eval_plus),
    declared("minus", eval_minus),
    declared("since", eval_since),
    declared("add_days", eval_add_days),
    declared("add_months", eval_add_months),
    declared("add_years", eval_add_years),
    declared("days_since", eval_days_since),
    declared("negate", eval_negate),
    declared("times", eval_times),
    declared("truncate", eval_truncate),
    declared("dates", eval_dates),
    declared("fmt", eval_fmt),
    clock("now", eval_now),
    clock("today", eval_today),
    clock("local_zone", eval_local_zone),
];

type Result<T> = std::result::Result<T, Diagnostic>;

fn fail(span: &Range<usize>, message: impl Into<String>) -> Diagnostic {
    Diagnostic::runtime(span.clone(), message)
}

/// The value an argument holds: the one written with `name`, else the
/// `position`th written without one.
fn argument(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    name: &str,
    position: usize,
) -> Result<Option<Value>> {
    let found = super::named(arguments, name).or_else(|| {
        arguments
            .iter()
            .filter(|argument| argument.name.is_none())
            .nth(position)
    });
    found
        .map(|argument| cx.evaluate(&argument.value))
        .transpose()
}

/// A value seen through `Option` and `Data`: the scalar it holds.
fn settled(value: Value) -> Value {
    match value {
        Value::Present(held) => settled(held.as_ref().clone()),
        Value::Data(tree) => match tree.as_ref() {
            crate::data::Data::Time(time) => Value::Time(*time),
            crate::data::Data::Str(text) => Value::Str(Arc::from(text.as_str())),
            crate::data::Data::Int(n) => Value::Int(*n),
            _ => Value::Data(tree),
        },
        other => other,
    }
}

fn time_of(value: Value, name: &str, span: &Range<usize>) -> Result<Time> {
    match settled(value) {
        Value::Time(time) => Ok(time),
        other => Err(fail(
            span,
            format!("`{name}` takes a time value, not {}", other.type_of()),
        )),
    }
}

fn text_of(value: Value, name: &str, span: &Range<usize>) -> Result<String> {
    match settled(value) {
        Value::Str(text) => Ok(text.to_string()),
        Value::Formatted(formatted) => Ok(formatted.text.clone()),
        other => Err(fail(
            span,
            format!("`{name}` takes a String, not {}", other.type_of()),
        )),
    }
}

fn int_of(value: Value, name: &str, span: &Range<usize>) -> Result<i64> {
    match settled(value) {
        Value::Int(n) => Ok(n),
        other => Err(fail(
            span,
            format!("`{name}` takes an Int, not {}", other.type_of()),
        )),
    }
}

fn required(value: Option<Value>, what: &str, name: &str, span: &Range<usize>) -> Result<Value> {
    value.ok_or_else(|| fail(span, format!("`{name}` needs {what}")))
}

fn int_argument(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    parameter: &str,
    position: usize,
    name: &str,
    span: &Range<usize>,
) -> Result<Option<i64>> {
    argument(cx, arguments, parameter, position)?
        .map(|value| int_of(value, name, span))
        .transpose()
}

fn time_argument(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    parameter: &str,
    name: &str,
    span: &Range<usize>,
) -> Result<Time> {
    let value = required(
        argument(cx, arguments, parameter, 0)?,
        &format!("`{parameter}`"),
        name,
        span,
    )?;
    time_of(value, name, span)
}

/// The `Zone` record of `name`.
fn zone_value(name: &str) -> Value {
    Value::Record(Arc::new(Record {
        ty: TypeRef::ZONE,
        fields: vec![("name".to_owned(), Value::Str(Arc::from(name)))],
        carrier: None,
    }))
}

/// The zone a `Zone` record names.
fn zone_of(value: Value, name: &str, span: &Range<usize>) -> Result<TimeZone> {
    let named = match settled(value) {
        Value::Record(record) => match record.field("name") {
            Some(Value::Str(text)) => text.to_string(),
            _ => return Err(fail(span, format!("`{name}` takes a Zone"))),
        },
        other => {
            return Err(fail(
                span,
                format!("`{name}` takes a Zone, not {}", other.type_of()),
            ));
        }
    };
    t::zone(&named).map_err(|message| fail(span, message))
}

fn zone_argument(
    cx: &mut dyn EvalCx,
    arguments: &[TypedArg],
    position: usize,
    name: &str,
    span: &Range<usize>,
) -> Result<Option<TimeZone>> {
    argument(cx, arguments, "zone", position)?
        .map(|value| zone_of(value, name, span))
        .transpose()
}

fn out_of_range(span: &Range<usize>, name: &str) -> Diagnostic {
    fail(
        span,
        format!("`{name}` leaves the representable range of time"),
    )
}

fn date(value: civil::Date) -> Value {
    Value::Time(Time::Date(value))
}

fn datetime(value: civil::DateTime) -> Value {
    Value::Time(Time::DateTime(value))
}

fn instant(value: Timestamp) -> Value {
    Value::Time(Time::Instant(value))
}

fn duration(value: SignedDuration) -> Value {
    Value::Time(Time::Duration(value))
}

/// A part of a value narrowed to the type jiff takes it as.
fn part<N: TryFrom<i64>>(value: i64, part: &str, name: &str, span: &Range<usize>) -> Result<N> {
    N::try_from(value).map_err(|_| fail(span, format!("`{name}`: {part} {value} is out of range")))
}

/// A constructor's text form: the type's ISO 8601 text, or an error naming
/// the text.
fn constructed(type_name: &str, text: &str, span: &Range<usize>) -> Result<Value> {
    Time::parse_as(type_name, text)
        .map(Value::Time)
        .ok_or_else(|| {
            let form = match type_name {
                "Date" => "an ISO 8601 date such as \"2026-10-24\"",
                "DateTime" => {
                    "an ISO 8601 date-time with no offset such as \"2026-10-24T09:30:00\""
                }
                "Instant" => "an RFC 3339 instant with an offset such as \"2026-10-24T07:30:00Z\"",
                _ => "an ISO 8601 duration without years or months such as \"PT1H30M\"",
            };
            let article = if type_name == "Instant" { "an" } else { "a" };
            fail(
                span,
                format!("{text:?} is not {article} {type_name}: write {form}"),
            )
        })
}

fn eval_date(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    if let Value::Str(text) = settled(input) {
        return constructed("Date", &text, &span);
    }
    let mut parts = [0_i64; 3];
    for (slot, name) in parts.iter_mut().zip(["year", "month", "day"]) {
        *slot = int_argument(cx, arguments, name, usize::MAX, "Date", &span)?
            .ok_or_else(|| fail(&span, format!("`Date` needs `{name}`")))?;
    }
    let [year, month, day] = parts;
    civil::Date::new(
        part(year, "year", "Date", &span)?,
        part(month, "month", "Date", &span)?,
        part(day, "day", "Date", &span)?,
    )
    .map(date)
    .map_err(|_| {
        fail(
            &span,
            format!("year {year}, month {month}, day {day} is not a date"),
        )
    })
}

fn eval_datetime(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let on = match settled(input) {
        Value::Str(text) => return constructed("DateTime", &text, &span),
        Value::Time(Time::Date(on)) => on,
        other => {
            return Err(fail(
                &span,
                format!(
                    "`DateTime` takes a String or a Date, not {}",
                    other.type_of()
                ),
            ));
        }
    };
    let mut parts = [0_i64; 4];
    for (slot, name) in parts
        .iter_mut()
        .zip(["hour", "minute", "second", "nanosecond"])
    {
        *slot = int_argument(cx, arguments, name, usize::MAX, "DateTime", &span)?.unwrap_or(0);
    }
    let [hour, minute, second, nanosecond] = parts;
    let time = civil::Time::new(
        part(hour, "hour", "DateTime", &span)?,
        part(minute, "minute", "DateTime", &span)?,
        part(second, "second", "DateTime", &span)?,
        part(nanosecond, "nanosecond", "DateTime", &span)?,
    )
    .map_err(|_| {
        fail(
            &span,
            format!("{hour:02}:{minute:02}:{second:02} and {nanosecond} nanoseconds is not a time of day"),
        )
    })?;
    Ok(datetime(on.to_datetime(time)))
}

fn eval_instant(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    if let Value::Str(text) = settled(input) {
        return constructed("Instant", &text, &span);
    }
    if let Some(seconds) =
        int_argument(cx, arguments, "unix_seconds", usize::MAX, "Instant", &span)?
    {
        return Timestamp::from_second(seconds)
            .map(instant)
            .map_err(|_| out_of_range(&span, "Instant"));
    }
    if let Some(millis) = int_argument(cx, arguments, "unix_millis", usize::MAX, "Instant", &span)?
    {
        return Timestamp::from_millisecond(millis)
            .map(instant)
            .map_err(|_| out_of_range(&span, "Instant"));
    }
    Err(fail(
        &span,
        "`Instant` needs text, `unix_seconds` or `unix_millis`",
    ))
}

fn eval_duration(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    if let Value::Str(text) = settled(input) {
        return constructed("Duration", &text, &span);
    }
    let mut nanos: i128 = 0;
    for (name, unit) in [
        ("days", 86_400_000_000_000_i128),
        ("hours", 3_600_000_000_000),
        ("minutes", 60_000_000_000),
        ("seconds", 1_000_000_000),
        ("millis", 1_000_000),
        ("nanos", 1),
    ] {
        if let Some(count) = int_argument(cx, arguments, name, usize::MAX, "Duration", &span)? {
            nanos += i128::from(count) * unit;
        }
    }
    t::duration_of_nanos(nanos)
        .map(duration)
        .ok_or_else(|| out_of_range(&span, "Duration"))
}

fn eval_zone(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let name = text_of(input, "Zone", &span)?;
    t::zone(&name).map_err(|message| fail(&span, message))?;
    Ok(zone_value(&name))
}

/// A `to_*` conversion: the value the text reads as, or `none`.
fn converted(type_name: &str, input: Value, name: &str, span: &Range<usize>) -> Result<Value> {
    let text = text_of(input, name, span)?;
    Ok(match Time::parse_as(type_name, text.trim()) {
        Some(time) => Value::Present(Arc::new(Value::Time(time))),
        None => Value::Absent(TypeRef {
            constructor: TypeConstructor(type_name_static(type_name)),
            args: Vec::new(),
        }),
    })
}

fn type_name_static(type_name: &str) -> &'static str {
    match type_name {
        "Date" => "Date",
        "DateTime" => "DateTime",
        "Instant" => "Instant",
        _ => "Duration",
    }
}

macro_rules! conversions {
    ($($eval:ident => $ty:literal, $name:literal;)*) => {
        $(fn $eval(
            _: &mut dyn EvalCx,
            input: Value,
            _: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value> {
            converted($ty, input, $name, &span)
        })*
    };
}

conversions! {
    eval_to_date => "Date", "to_date";
    eval_to_datetime => "DateTime", "to_datetime";
    eval_to_instant => "Instant", "to_instant";
    eval_to_duration => "Duration", "to_duration";
}

fn parsed(
    type_name: &str,
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    name: &str,
    span: &Range<usize>,
) -> Result<Value> {
    // With no pattern the reading is the ISO one of the matching `to_*`.
    let Some(pattern) = argument(cx, arguments, "pattern", 0)? else {
        return converted(type_name, input, name, span);
    };
    let text = text_of(input, name, span)?;
    let pattern = text_of(pattern, name, span)?;
    Ok(
        match t::parse_pattern(type_name, &text, &pattern).map_err(|m| fail(span, m))? {
            Some(time) => Value::Present(Arc::new(Value::Time(time))),
            None => Value::Absent(TypeRef {
                constructor: TypeConstructor(type_name_static(type_name)),
                args: Vec::new(),
            }),
        },
    )
}

macro_rules! patterns {
    ($($eval:ident => $ty:literal, $name:literal;)*) => {
        $(fn $eval(
            cx: &mut dyn EvalCx,
            input: Value,
            arguments: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value> {
            parsed($ty, cx, input, arguments, $name, &span)
        })*
    };
}

patterns! {
    eval_parse_date => "Date", "parse_date";
    eval_parse_datetime => "DateTime", "parse_datetime";
    eval_parse_instant => "Instant", "parse_instant";
}

fn eval_at(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let Time::Instant(at) = time_of(input, "at", &span)? else {
        return Err(fail(&span, "`at` reads an Instant"));
    };
    let zone = zone_argument(cx, arguments, 0, "at", &span)?
        .ok_or_else(|| fail(&span, "`at` needs a zone"))?;
    Ok(datetime(zone.to_datetime(at)))
}

fn eval_in_zone(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let Time::DateTime(reading) = time_of(input, "in_zone", &span)? else {
        return Err(fail(&span, "`in_zone` reads a DateTime"));
    };
    let zone = zone_argument(cx, arguments, 0, "in_zone", &span)?
        .ok_or_else(|| fail(&span, "`in_zone` needs a zone"))?;
    t::in_zone(reading, &zone)
        .map(instant)
        .map_err(|message| fail(&span, message))
}

fn eval_offset(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let zone = zone_of(input, "offset", &span)?;
    let Time::Instant(at) = time_argument(cx, arguments, "at", "offset", &span)? else {
        return Err(fail(&span, "`offset` is read at an Instant"));
    };
    let seconds = zone.to_offset(at).seconds();
    Ok(duration(SignedDuration::from_secs(i64::from(seconds))))
}

/// The date a `Date` or `DateTime` falls on.
fn calendar_date(input: Value, name: &str, span: &Range<usize>) -> Result<civil::Date> {
    match time_of(input, name, span)? {
        Time::Date(on) => Ok(on),
        Time::DateTime(at) => Ok(at.date()),
        other => Err(fail(
            span,
            format!(
                "`{name}` reads a Date or a DateTime, not {}",
                other.type_name()
            ),
        )),
    }
}

/// A `DateTime`'s reading.
fn wall_clock(input: Value, name: &str, span: &Range<usize>) -> Result<civil::DateTime> {
    match time_of(input, name, span)? {
        Time::DateTime(at) => Ok(at),
        other => Err(fail(
            span,
            format!("`{name}` reads a DateTime, not {}", other.type_name()),
        )),
    }
}

macro_rules! date_parts {
    ($($eval:ident => $name:literal, $part:expr;)*) => {
        $(fn $eval(
            _: &mut dyn EvalCx,
            input: Value,
            _: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value> {
            let on = calendar_date(input, $name, &span)?;
            let part: fn(civil::Date) -> i64 = $part;
            Ok(Value::Int(part(on)))
        })*
    };
}

date_parts! {
    eval_year => "year", |d| i64::from(d.year());
    eval_month => "month", |d| i64::from(d.month());
    eval_day => "day", |d| i64::from(d.day());
    eval_weekday => "weekday", |d| i64::from(d.weekday().to_monday_one_offset());
    eval_day_of_year => "day_of_year", |d| i64::from(d.day_of_year());
    eval_iso_week => "iso_week", |d| i64::from(d.iso_week_date().week());
    eval_iso_year => "iso_year", |d| i64::from(d.iso_week_date().year());
}

macro_rules! time_parts {
    ($($eval:ident => $name:literal, $part:expr;)*) => {
        $(fn $eval(
            _: &mut dyn EvalCx,
            input: Value,
            _: &[TypedArg],
            span: Range<usize>,
        ) -> Result<Value> {
            let at = wall_clock(input, $name, &span)?;
            let part: fn(civil::DateTime) -> i64 = $part;
            Ok(Value::Int(part(at)))
        })*
    };
}

time_parts! {
    eval_hour => "hour", |d| i64::from(d.hour());
    eval_minute => "minute", |d| i64::from(d.minute());
    eval_second => "second", |d| i64::from(d.second());
    eval_nanosecond => "nanosecond", |d| i64::from(d.subsec_nanosecond());
}

fn eval_date_of(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    Ok(date(wall_clock(input, "date", &span)?.date()))
}

/// An instant's nanoseconds since the epoch, floored to `unit`.
fn unix(input: Value, name: &str, unit: i128, span: &Range<usize>) -> Result<Value> {
    let Time::Instant(at) = time_of(input, name, span)? else {
        return Err(fail(span, format!("`{name}` reads an Instant")));
    };
    i64::try_from(at.as_nanosecond().div_euclid(unit))
        .map(Value::Int)
        .map_err(|_| Diagnostic::overflow(span.clone()))
}

fn eval_unix_seconds(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    unix(input, "unix_seconds", 1_000_000_000, &span)
}

fn eval_unix_millis(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    unix(input, "unix_millis", 1_000_000, &span)
}

fn length(input: Value, name: &str, span: &Range<usize>) -> Result<SignedDuration> {
    match time_of(input, name, span)? {
        Time::Duration(length) => Ok(length),
        other => Err(fail(
            span,
            format!("`{name}` reads a Duration, not {}", other.type_name()),
        )),
    }
}

fn eval_total_seconds(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let nanos = length(input, "total_seconds", &span)?.as_nanos();
    let sign = if nanos < 0 { "-" } else { "" };
    let magnitude = nanos.unsigned_abs();
    let fraction = format!("{:09}", magnitude % 1_000_000_000);
    let fraction = fraction.trim_end_matches('0');
    let text = if fraction.is_empty() {
        format!("{sign}{}", magnitude / 1_000_000_000)
    } else {
        format!("{sign}{}.{fraction}", magnitude / 1_000_000_000)
    };
    crate::decimal::Decimal::parse(&text)
        .map(Value::Decimal)
        .ok_or_else(|| Diagnostic::overflow(span.clone()))
}

fn eval_total_millis(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    i64::try_from(length(input, "total_millis", &span)?.as_millis())
        .map(Value::Int)
        .map_err(|_| Diagnostic::overflow(span.clone()))
}

/// `self` moved by `by`, forward or back.
fn moved(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    name: &str,
    forward: bool,
    span: &Range<usize>,
) -> Result<Value> {
    let by = match time_argument(cx, arguments, "by", name, span)? {
        Time::Duration(by) if forward => by,
        Time::Duration(by) => by.checked_neg().ok_or_else(|| out_of_range(span, name))?,
        other => {
            return Err(fail(
                span,
                format!("`{name}` moves by a Duration, not {}", other.type_name()),
            ));
        }
    };
    match time_of(input, name, span)? {
        Time::Instant(at) => at.checked_add(by).map(instant),
        Time::DateTime(at) => at.checked_add(by).map(datetime),
        other => {
            return Err(fail(
                span,
                format!(
                    "`{name}` moves an Instant or a DateTime, not {}",
                    other.type_name()
                ),
            ));
        }
    }
    .map_err(|_| out_of_range(span, name))
}

fn eval_plus(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    moved(cx, input, arguments, "plus", true, &span)
}

fn eval_minus(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    moved(cx, input, arguments, "minus", false, &span)
}

fn eval_since(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let earlier = time_argument(cx, arguments, "earlier", "since", &span)?;
    match (time_of(input, "since", &span)?, earlier) {
        (Time::Instant(at), Time::Instant(earlier)) => Ok(duration(at.duration_since(earlier))),
        (Time::DateTime(at), Time::DateTime(earlier)) => Ok(duration(at.duration_since(earlier))),
        _ => Err(fail(&span, "`since` takes two Instants or two DateTimes")),
    }
}

/// A date or date-time moved by a calendar span.
fn calendar_moved(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    name: &str,
    span_of: fn(i64) -> std::result::Result<Span, jiff::Error>,
    span: &Range<usize>,
) -> Result<Value> {
    let n = int_argument(cx, arguments, "n", 0, name, span)?
        .ok_or_else(|| fail(span, format!("`{name}` needs `n`")))?;
    let by = span_of(n).map_err(|_| out_of_range(span, name))?;
    match time_of(input, name, span)? {
        Time::Date(on) => on.checked_add(by).map(date),
        Time::DateTime(at) => at.checked_add(by).map(datetime),
        other => {
            return Err(fail(
                span,
                format!(
                    "`{name}` moves a Date or a DateTime, not {}",
                    other.type_name()
                ),
            ));
        }
    }
    .map_err(|_| out_of_range(span, name))
}

fn eval_add_days(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    calendar_moved(
        cx,
        input,
        arguments,
        "add_days",
        |n| Span::new().try_days(n),
        &span,
    )
}

fn eval_add_months(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    calendar_moved(
        cx,
        input,
        arguments,
        "add_months",
        |n| Span::new().try_months(n),
        &span,
    )
}

fn eval_add_years(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    calendar_moved(
        cx,
        input,
        arguments,
        "add_years",
        |n| Span::new().try_years(n),
        &span,
    )
}

fn eval_days_since(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (Time::Date(on), Time::Date(earlier)) = (
        time_of(input, "days_since", &span)?,
        time_argument(cx, arguments, "earlier", "days_since", &span)?,
    ) else {
        return Err(fail(&span, "`days_since` takes two Dates"));
    };
    let days = earlier
        .until(on)
        .map_err(|_| out_of_range(&span, "days_since"))?
        .get_days();
    Ok(Value::Int(i64::from(days)))
}

fn eval_negate(
    _: &mut dyn EvalCx,
    input: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    length(input, "negate", &span)?
        .checked_neg()
        .map(duration)
        .ok_or_else(|| out_of_range(&span, "negate"))
}

fn eval_times(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let nanos = length(input, "times", &span)?.as_nanos();
    let n = int_argument(cx, arguments, "n", 0, "times", &span)?
        .ok_or_else(|| fail(&span, "`times` needs `n`"))?;
    nanos
        .checked_mul(i128::from(n))
        .and_then(t::duration_of_nanos)
        .map(duration)
        .ok_or_else(|| out_of_range(&span, "times"))
}

fn eval_truncate(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let unit = super::named(arguments, "unit")
        .or_else(|| arguments.iter().find(|argument| argument.name.is_none()))
        .and_then(|argument| Unit::named(argument.value.ty.constructor.0))
        .ok_or_else(|| fail(&span, "`truncate` needs a unit: `Day`, `Month`, ..."))?;
    match time_of(input, "truncate", &span)? {
        Time::Date(on) => Ok(date(t::truncate_date(on, unit))),
        Time::DateTime(at) => Ok(datetime(t::truncate_datetime(at, unit))),
        Time::Instant(at) => {
            let zone = zone_argument(cx, arguments, 1, "truncate", &span)?.unwrap_or(TimeZone::UTC);
            let start = t::truncate_datetime(zone.to_datetime(at), unit);
            t::in_zone(start, &zone)
                .map(instant)
                .map_err(|message| fail(&span, message))
        }
        Time::Duration(_) => Err(fail(
            &span,
            "`truncate` reads a Date, a DateTime or an Instant",
        )),
    }
}

fn eval_dates(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (Time::Date(start), Time::Date(end)) = (
        time_of(input, "dates", &span)?,
        time_argument(cx, arguments, "end", "dates", &span)?,
    ) else {
        return Err(fail(&span, "`dates` takes two Dates"));
    };
    let days: Vec<Value> = start
        .series(Span::new().days(1))
        .take_while(|on| *on < end)
        .map(date)
        .collect();
    Ok(Value::list(days, TypeRef::DATE))
}

fn eval_fmt(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let value = time_of(input, "fmt", &span)?;
    let pattern = required(
        argument(cx, arguments, "pattern", 0)?,
        "a pattern",
        "fmt",
        &span,
    )?;
    let pattern = text_of(pattern, "fmt", &span)?;
    let zone = zone_argument(cx, arguments, 1, "fmt", &span)?.unwrap_or(TimeZone::UTC);
    t::format(&value, &pattern, &zone)
        .map(|text| Value::Str(Arc::from(text)))
        .map_err(|message| fail(&span, message))
}

fn evaluation_clock(span: &Range<usize>) -> Result<(Timestamp, TimeZone, String)> {
    t::evaluation_clock().map_err(|message| fail(span, message))
}

fn eval_now(_: &mut dyn EvalCx, _: Value, _: &[TypedArg], span: Range<usize>) -> Result<Value> {
    Ok(instant(evaluation_clock(&span)?.0))
}

fn eval_today(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    let (now, local, _) = evaluation_clock(&span)?;
    let zone = match settled(input) {
        record @ Value::Record(_) => zone_of(record, "today", &span)?,
        _ => zone_argument(cx, arguments, 0, "today", &span)?.unwrap_or(local),
    };
    Ok(date(zone.to_datetime(now).date()))
}

fn eval_local_zone(
    _: &mut dyn EvalCx,
    _: Value,
    _: &[TypedArg],
    span: Range<usize>,
) -> Result<Value> {
    Ok(zone_value(&evaluation_clock(&span)?.2))
}
