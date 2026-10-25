# Results

`Result<T, E>` is a computation that may have failed: `ok(v)` holding a
`T`, or `err(e)` holding an `E` that says how. The possibility of failure is
in the type, so a caller cannot forget it, and the error is a value, so a
program can say what it is.

## Building and matching

`ok(v)` and `err(e)` build a result, and `ok(p)` and `err(p)` match one. An
error is any type, here a record the program declares for it:

```hql
type ParseError { input : String, reason : String }

fn parse_age(text: String) : Result<Int, ParseError> = match text.to_int() {
    some(n) given n >= 0 => ok(n),
    some(_) => err(ParseError(input = text, reason = "negative")),
    none => err(ParseError(input = text, reason = "not a number")),
}

["42", "-3", "forty"] | map(parse_age)
```

```text
[ok(42), err(ParseError(input = "-3", reason = "negative")), err(ParseError(input = "forty", reason = "not a number"))]
```

A `match` over a result covers both cases, and a missing one is a type
error, as over any union:

```hql
type ParseError { input : String, reason : String }

fn parse_age(text: String) : Result<Int, ParseError> = match text.to_int() {
    some(n) given n >= 0 => ok(n),
    some(_) => err(ParseError(input = text, reason = "negative")),
    none => err(ParseError(input = text, reason = "not a number")),
}

fn describe(r: Result<Int, ParseError>) : String = match r {
    ok(n) => fmt("age {}", n),
    err(e) => fmt("cannot read {}: {}", e.input, e.reason),
}

["42", "-3", "forty"] | map(parse_age) | map(describe)
```

```text
["age 42", "cannot read -3: negative", "cannot read forty: not a number"]
```

## The ways out

The ways out of a `Result` mirror `Option`'s. `map` transforms the value an
`ok` holds and leaves an `err` as it is. `and_then` chains a second
computation that may fail. `unwrap_or` leaves the result with a value for
the error case:

```hql
fn parse(text: String) : Result<Int, String> = match text.to_int() {
    some(n) => ok(n),
    none => err(fmt("not a number: {}", text)),
}

[
    parse("7").map(n => n * 2).unwrap_or(0),
    parse("x").map(n => n * 2).unwrap_or(0),
    parse("7").and_then(n => parse("1").map(m => n + m)).unwrap_or(0)
]
```

```text
[14, 0, 8]
```

`map_err` transforms the error and leaves an `ok` as it is. `ok()` and
`err()` give either side as an `Option`, the other side being `none`:

```hql
fn parse(text: String) : Result<Int, String> = match text.to_int() {
    some(n) => ok(n),
    none => err("no"),
}
[parse("4").ok(), parse("x").ok(), parse("x").err()]
```

```text
[4, none, "no"]
```

Over a collection of results, `oks()` keeps the values of the successes and
`errs()` the errors of the failures, each at its own type:

```hql
fn parse(text: String) : Result<Int, String> = match text.to_int() {
    some(n) => ok(n),
    none => err(fmt("not a number: {}", text)),
}
rs = ["4", "x", "9"] | map(parse)
[rs | filter(r => r.is_ok()) | size, rs | oks(), rs | errs()]
```

```text
[2, [4, 9], ["not a number: x"]]
```

## There is no unwrap

Every way out of a `Result` says what the error case means: a `match` names
it, `unwrap_or` replaces it, `ok()` makes it an absence. There is no
operation that takes the value and stops the program otherwise.
`parse("4").unwrap()` is a name error, since no such function exists. A
failure the program cannot handle as a value is an evaluation failure,
raised by the runtime and not caught by HQL code; the two are kept apart,
and a `Result` is for the failures a program expects of its input.

## Where results come from

Parsing, decoding and validation are the operations whose failure is an
expected consequence of their input, and each has a `Result` form.
`validate(T)` reads a tree as a contract and gives `Result<T, Invalid>`,
with every reason for a failure inside the error; over a collection it is
elementwise, and `oks()`, `errs()` and `fork` route the two sides.
[Validation as a value](../17-advanced-data-typing/validation-as-a-value.md)
is that pattern in full.
