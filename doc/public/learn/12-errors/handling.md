# Failures as data

A data failure is a value of the program. Its type says it may be a failure,
the checker holds every use of it to that type, and the run writes no
diagnostic for it.

## `Result`

`Result<T, E>` is `ok(v)` or `err(e)`. A function that may fail declares it
in its result type, and a `match` covers both cases:

<!-- define: parse -->
```hql
fn parse(text: String) : Result<Int, String> = match text.to_int() {
    some(n) => ok(n),
    none => err(fmt("not a number: {}", text)),
}

["4", "x", "15"] | map(parse)
```

```text
[ok(4), err("not a number: x"), ok(15)]
```

A `Result` is not its value. `parse("4") + 1` is a type error, found before
anything runs:

<!-- run: with parse -->
```hql
parse("4") + 1
```

```text
check error[type-mismatch]: addition requires two Int, two Float or two Decimal operands, or two values of a type implementing `Add`, not Result<Int, String>
 --> <expression>:7:1
  |
7 | parse("4") + 1
  | ^^^^^^^^^^
```

`unwrap_or` gives the value or a default, `map_err` changes the error, and
`oks()` and `errs()` keep one side of a sequence of results:

<!-- run: with parse -->
```hql
results = ["4", "x", "15"] | map(parse)
{
    total: results | oks() | sum,
    rejected: results | errs(),
    fallback: parse("x").unwrap_or(0),
}
```

```text
{fallback: 0, rejected: ["not a number: x"], total: 19}
```

## `validate`: a contract read as a value

`x | validate(T)` reads `x` as the declared type `T` and gives
`Result<T, Invalid>`. An `Invalid` carries the contract, the value and one
`Violation` per broken rule, each with its `path`, `kind`, `expected` and
`actual`:

```hql
type User { name : String, age : Int }

{name: "bob", age: "forty"} | validate(User)
```

```text
err(Invalid(contract = "User", value = {age: "forty", name: "bob"}, violations = [Violation(contract = "User", path = ["age"], kind = "type", rule = "User.age.type", text = "\"forty\" is not Int", expected = "Int", actual = "forty", severity = "error", message = "field `age`: \"forty\" is not Int")]))
```

Over many rows, `errs()` keeps the rejected ones as data the program can
count, sort or write out:

```hql
type User { name : String, age : Int }

rows = [{name: "ada", age: 36}, {name: "bob", age: "forty"}]
rows
| map(row => row | validate(User))
| errs()
| map(invalid => invalid.violations | map(v => v.message))
```

```text
[["field `age`: \"forty\" is not Int"]]
```

The run above writes no diagnostic, and its exit status is `0`. A `Violation`
with `severity = "warning"` is data in the same way.

## `x | T`: a contract read as a requirement

Without `validate`, `x | T` requires `x` to read as `T`. A value that does
not is a failure of the execute phase, written as a diagnostic, and the run
ends:

```hql
type User { name : String, age : Int }

Data("{\"name\": \"bob\", \"age\": \"forty\"}", Json) | User
```

```text
execute error[contract-violation]: this does not read as User: field `age`: "forty" is not Int
 --> <expression>:3:1
  |
3 | Data("{\"name\": \"bob\", \"age\": \"forty\"}", Json) | User
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = expected: User
```

The diagnostic carries the same `Violation`s `validate` would have given, as
its context. They remain the diagnostic's: no operation reads them back into
the program. Which of the two forms to write is the program's decision, at the
call site: `validate` where a rejected value is part of the result, `| T`
where it ends the run.

## What is next

[Diagnostics](diagnostics.md) describes the other side: what a diagnostic
carries, where it goes, and when a run stops.
