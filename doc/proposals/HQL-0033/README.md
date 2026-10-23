# HQL-0033: Errors as values and error handling

**Status**: accepted
**Created**: 2026-10-23
**Source**: [HQL-0032](../HQL-0032/README.md), which leaves the form of a
validation result and of a diagnostic to this record; tracked by
[issue 0043](../../issues/0043-contract-validation.md)

## Abstract

An error in HQL is a value. A computation that can fail returns
`Result<T, E>`, and its failure `E` is a declared type. HQL has no `try`, no
`catch` and no exception: a failure travels as an `err(e)` through the same
pipelines, `match` arms and operations as any other value, and a program says
what happens to it by the steps it writes. Validation is the first producer.
`validate(T)` reads a value through a contract and gives
`Result<T, Invalid>`, where `Invalid` and each `Violation` in it are
`DataContract` values that the standard library declares. Because they are
data, a failure formats as `Json`, `Text` or any other format, and it is
written to a stream like any other message: a channel, a file, or a log
topic. The runtime error that `x | T` raises today remains as the fail-fast
form. It is the one failure a program does not handle.

## Motivation

Today `data | T` either gives a `T` or stops the program with a message that
names every mismatch. That message is text. A program cannot read it, route
it or count it. HQL-0032 asks for structured diagnostics, error
accumulation, severity, rule identity and stream policies. All of them need
the failure to be a value with a declared shape.

- A pipeline over a stream must keep going past one bad record, and the
  record and the reason must go somewhere a consumer can read them.
- An IDE, a log and a metrics step need the path, the rule and the value
  that broke it as fields, not as a sentence to parse.
- HQL already has `Result<T, E>` with `ok` and `err` as constructors and
  patterns, and `map`, `map_err`, `and_then`, `or_else` and `unwrap_or` over
  it (see [the result example](../../../examples/mighty-types/queries/08-result.hql)).
  Error handling is completed by building on it, not by adding a second
  mechanism beside it.

## Non-goals

- Exceptions, `try`/`catch`, or any control flow that skips steps of a
  pipeline. A failure is never thrown.
- An `unwrap`. The `result` module's rule stands: every way out of a
  `Result` says what a failure means.
- Dataset-level constraints, contract versioning and migrations, which stay
  with HQL-0032.

## Specification

### 1. Two kinds of failure

HQL distinguishes the failure a program handles from the one that stops it.

- A **recoverable** failure MUST be a value: an `err(e)` of a
  `Result<T, E>`, with `E` a declared type. Every standard operation whose
  failure depends on its input data (decoding, interpretation, contract
  validation, parsing) MUST have a form that returns a `Result`.
- An **unrecoverable** failure stops evaluation with a diagnostic and has no
  value: a broken invariant of the runtime, an exhausted resource, or a step
  written to fail fast. A program MUST NOT be able to intercept one. A program
  that wants to handle a failure uses the `Result` form instead.
- `x | T` stays the fail-fast form: where `x` does not read as `T` or breaks
  one of `T`'s rules, evaluation stops with a diagnostic that lists every
  violation. `x | validate(T)` is the same reading with the failure returned.

The split keeps every handled failure visible in the types. A `Result` in a
signature is the whole statement that something can fail.

### 2. `validate(T)`

`validate` reads a value through a declared type and returns the outcome.

```hql
fn validate<T>(self: Data, contract: Type<T>) : Result<T, Invalid>
```

- Over a collection or a stream it is elementwise. `Seq<Data> | validate(T)`
  is a `Seq<Result<T, Invalid>>`, and a `Stream` stays a `Stream`.
- It follows HQL-0032's evaluation phases: structural reading, defaults,
  field rules, logical rules, cross-field rules. It MUST collect every
  violation of one value in one pass and MUST NOT stop at the first.
- `T` may be any record type. A plain record has only structural
  violations, and a contract adds the violations of its rules.
- `ValidationResult<T>`, the name HQL-0032 uses, is `Result<T, Invalid>`.
  No separate type is declared.

`validate` the operation and `validate` the rule keyword do not collide. The
keyword is read only inside a contract body, at the head of a rule.

### 3. `Violation` and `Invalid`

The standard library declares the shape of a failure as a `DataContract`.
Every producer of validation failures uses these types.

```hql
// One reason a value is not a T.
type Violation : DataContract = {
    contract: String         // the type read as, "UserData"
    path: List<String>       // ["profile", "address", "postal_code"]; [] is the value itself
    kind: String             // "missing", "undeclared", "type" or "rule"
    rule: String             // the rule's identity, as Rule identity states it
    text: String             // the rule as written: "require age? >= 0"
    expected?: String        // "Int", or the rule's condition
    actual?: Data            // the value found, absent where there is none
    severity: String = "error"
    message: String          // one sentence, rendered from the fields above

    require kind == "missing" or kind == "undeclared" or kind == "type" or kind == "rule"
    require severity == "error" or severity == "warning"
}

// A value that did not read as a contract, and every reason why.
type Invalid : DataContract = {
    contract: String
    value: Data              // the value as read, before defaults
    violations: List<Violation>

    require violations.size() > 0
}
```

- `path` is a list of segments, not a dotted string. A list element is
  named by its position as text, `"3"`. Rendering a path is the
  renderer's choice.
- `actual` is `Data`, because the value that broke a rule need not be of the
  field's declared type. A `type` violation is exactly that case.
- `message` is derived from the other fields. Programs SHOULD read the fields
  and use `message` only for display.
- The two types are contracts, so their own rules hold for every `Invalid`
  the runtime produces. They are open, so a later producer MAY add fields
  (`source`, `position`, `time`) without changing the declaration readers
  rely on.

### 4. Severity

A violation is an `error` or a `warning`.

- An `error` makes the value invalid. `validate` returns `err(Invalid)` when
  at least one error is found.
- A `warning` does not invalidate the value. A rule written `warn P`
  instead of `require P` gives warnings:

  ```hql
  type UserData : DataContract = {
      age?: Int
      require age? >= 0            // an error
      warn age? < 120 as plausible_age   // a warning
  }
  ```

  `x | T` does not stop for a warning, and `validate` gives `ok` for a
  value whose only violations are warnings. *Placeholder:* the place a
  valid value's warnings are reported.

### 5. Rule identity

Every violation names the rule it broke by a stable identity.

- A structural violation's identity is its kind and path:
  `"UserData.age.type"`, `"UserData.id.missing"`,
  `"Point.z.undeclared"`.
- A contract rule's identity is `"<Contract>.<name>"`. A rule MAY be named
  where it is written, `require id != "" as id_present`, and a named rule
  keeps its identity when its condition is rewritten. An unnamed rule's
  identity is `"<Contract>.<n>"`, its position among the contract's rules
  counted from 1. Inserting a rule before it changes that identity. Naming
  the rule is how a consumer gets an identity that does not change.
- `validate f with V` is named `"<Contract>.<f>.<V>"` unless named
  otherwise: `"UserData.country.is_country_code"`.
- `as name` follows the rule, whatever its keyword: `require P as name`,
  `warn P as name`, `validate f with V as name`.
- An inherited rule keeps the identity of the contract that declares it.

`text` keeps the rule exactly as written, so a reader sees what broke without
looking up the identity.

### 6. Handling a result

A `Result` is handled with the operations it already has, plus two steps
over collections of results.

```hql
elementwise fn oks<F<_> : Collection<_>, T, E>(self: F<Result<T, E>>) : F<T>
elementwise fn errs<F<_> : Collection<_>, T, E>(self: F<Result<T, E>>) : F<E>
```

`oks` and `errs` are to `Result` what `somes` is to `Option`. A program
chooses its policy for invalid values by the steps it writes. The policies
HQL-0032 lists are pipelines, not settings. *Placeholder:* how one stream
is split into its valid and its invalid values, the dead-letter case
below, is refined with stream policies; the `tee` form shows the intent,
not the syntax.


```hql
users = source(file("users.jsonl"), Json)

// fail fast: the first invalid value stops the program
users | UserData

// skip invalid values
users | validate(UserData) | oks()

// dead-letter: invalid values go to a channel, valid ones go on
users
| validate(UserData)
| tee rejected = errs() | sink(Channel<Invalid>("rejected"))
| oks()

// collect: every failure of a finite input, as one list
users | validate(UserData) | errs() | List

// one value, handled in a match
match d | validate(UserData) {
    ok(user) => user.name,
    err(e) => fmt("{} violations of {}", e.violations.size(), e.contract),
}
```

A threshold policy, which fails a stream once too large a share of its
elements is invalid, is a reduction over `errs()` and is specified with
stream reductions.

### 7. Errors on a stream

An `Invalid` is an ordinary value. It is written and read as one.

- `format(e, Json)` gives its JSON. `format(e, Text)` gives its `message`
  lines. `sink(errors, endpoint, Json)` writes one document per failure.
- A failure written to an endpoint is read back with its contract:
  `source(file("rejected.jsonl"), Json) | Invalid` reads it, and
  `| validate(Invalid)` checks it. A consumer in another program relies on
  the declared shape, not on the producer's code.
- A `Channel<Invalid>` holds failures in memory for one execution scope. A
  log endpoint, such as a Kafka topic once HQL-0021 provides one, carries
  them between programs. In both cases the failure is the message's
  `value`, and its `key`, `time` and `headers` are those of HQL-0021's
  `Message<T>`.

A failure is reported by writing it, which makes it a step of the program
and visible in its effects. Logging a failure is a separate step and does
not handle it.

### 8. Diagnostics of the fail-fast form

`x | T` and the runtime error it raises carry the same content as an
`Invalid`.

- The diagnostic MUST list every violation of the value, each with its path
  and, for a rule, the rule's text as written:

  ```text
  this does not read as UserData: rule `require age? >= 0` does not hold
  ```
- The checker and runtime diagnostics stay text for the terminal and the
  IDE. Their structured form is the `Invalid` that `validate` would give,
  and a host MAY attach it.

## Backwards Compatibility

Nothing written today changes meaning. `x | T` keeps failing fast, with the
same message plus rule violations. `validate`, `oks`, `errs`, `Violation`
and `Invalid` are new names. `validate` is a function name only outside a
contract body. A program that binds one of these names shadows it, as with
any prelude name.

## Reference Implementation

- `std/validation.hql`: `Violation`, `Invalid`, and the declarations of
  `validate`, `oks` and `errs`, in the prelude.
- `src/types/value.rs`: the structural mismatches become `Violation` values
  instead of text.
- `src/evaluation.rs`: contract rules report `Violation` values, and
  `validate` wraps the outcome in `ok` or `err`.
- `examples/structural/`: a case per policy in the handling section, and
  one that writes failures as JSON lines and reads them back as `Invalid`.

## Test Plan

- One value with a missing field, a mistyped field, an undeclared field and
  a broken rule gives one `Invalid` with four violations, in declaration
  order.
- Each identity form in the rule-identity section, including an inherited
  rule and a named rule.
- `oks` and `errs` over a `List`, a `Seq` and a `Stream`, the stream left
  unforced.
- An `Invalid` written with `sink(…, Json)` reads back with `| Invalid`.
- `x | T` reports the same violations as `x | validate(T)`.

```bash
cargo test --locked --test structural --test corpus
```

## Changelog

- 2026-10-23: proposed
- 2026-10-23: accepted: two reading modes, `warn P` for a warning, `as name`
  for a rule's identity; splitting a stream into valid and invalid values
  stays a placeholder
