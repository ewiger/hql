//! The extension of the `hello` package: one Rust function per name its
//! contract, `modules/hello/mod.hql`, declares.

#![deny(unsafe_code)]

use hql::ext::{self, Diagnostic, EvalCx, Operation, Span, TypeRef, TypedArg, Value};

/// The text greeting a name.
fn hello(name: &str, punctuation: &str) -> String {
    format!("Hello, {name}{punctuation}")
}

/// The input of a call as text.
fn text(input: &Value, span: &Span) -> Result<String, Diagnostic> {
    match input {
        Value::Str(text) => Ok(text.to_string()),
        _ => Err(ext::failure(span.clone(), "expected a `String`")),
    }
}

/// `fn greet(name: String) : String`
fn greet(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Span,
) -> Result<Value, Diagnostic> {
    Ok(Value::Str(hello(&text(&input, &span)?, "!").into()))
}

/// `fn Greeting(name: String, *, punctuation: String = "!") : Greeting`
fn greeting(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Span,
) -> Result<Value, Diagnostic> {
    // An argument the call does not write is absent: the default is here.
    let punctuation = match ext::named(arguments, "punctuation") {
        Some(argument) => text(&ext::value_of(cx, argument)?, &span)?,
        None => "!".to_owned(),
    };
    let said = hello(&text(&input, &span)?, &punctuation);
    let length = i64::try_from(said.chars().count())
        .map_err(|_| ext::failure(span.clone(), "the greeting is too long"))?;
    Ok(ext::record(
        ext::ty("Greeting", []),
        [
            ("text", Value::Str(said.into())),
            ("length", Value::Int(length)),
        ],
    ))
}

/// `fn shout(self: String) : String` and `fn shout(self: Greeting) : String`:
/// every form of a name is one operation, which reads what it was given.
fn shout(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Span,
) -> Result<Value, Diagnostic> {
    let said = match &input {
        Value::Record(record) => match record.field("text") {
            Some(held) => text(held, &span)?,
            None => return Err(ext::failure(span, "expected a `Greeting`")),
        },
        other => text(other, &span)?,
    };
    Ok(Value::Str(said.to_uppercase().into()))
}

/// `fn total<T>(self: List<T>, weight: T -> Int) : Int`
fn total(
    cx: &mut dyn EvalCx,
    input: Value,
    arguments: &[TypedArg],
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::List(elements, _) = input else {
        return Err(ext::failure(span, "expected a `List`"));
    };
    let Some(weight) = ext::positional(arguments, 0) else {
        return Err(ext::failure(span, "`total` takes a function"));
    };
    let mut sum = 0_i64;
    for element in elements.iter() {
        // The function is the program's: the running `hql` applies it.
        match cx.apply_lambda(weight, element.clone())? {
            Value::Int(weighed) => {
                sum = sum
                    .checked_add(weighed)
                    .ok_or_else(|| ext::failure(span.clone(), "numeric overflow"))?;
            }
            _ => return Err(ext::failure(span, "`weight` gives an `Int`")),
        }
    }
    Ok(Value::Int(sum))
}

/// `fn fail(message: String) : String`
fn fail(
    _cx: &mut dyn EvalCx,
    input: Value,
    _arguments: &[TypedArg],
    span: Span,
) -> Result<Value, Diagnostic> {
    panic!("{}", text(&input, &span)?)
}

/// The operations of module `hello`, one per declared name.
static HELLO: &[Operation] = &[
    hql::native!("Greeting", greeting),
    hql::native!("greet", greet),
    hql::native!("shout", shout),
    hql::native!("total", total),
    hql::native!("fail", fail),
];

hql::extension!(ext::extension(
    "hello",
    env!("CARGO_PKG_VERSION"),
    &[ext::provided("hello", HELLO)],
));

/// What `total` gives over a list of `Int`s, for the crate's own tests.
#[must_use]
pub fn element() -> TypeRef {
    TypeRef::INT
}
