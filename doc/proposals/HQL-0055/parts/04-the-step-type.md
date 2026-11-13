# HQL-0055: the type `Step`

Companion to [HQL-0055](../README.md). It records why `Unit` becomes `Step`,
where `Unit` is used, and how the rename lands in 0.6.0.

## Why `Step`

`Unit` was named for the atomic unit of computation. On the page, the word
reads as something else: a reader of the book cannot tell whether it is a
type, a value, a missing value or a status, and it reads as `none` with a
capital letter (issue 0063).

The candidates weighed:

| Name | For | Against |
| --- | --- | --- |
| `Step` | The language's own word for a unit of its program: a step ran, and that is what it gives | Can read as "returns a step", a deferred computation |
| `Done` | Reads as a verdict in a signature and in printed output | A second word beside *step* for the same thing |
| `Effect` | Names the interaction | Names what is waiting to happen, not that it happened; an effect is a property, not a value (see [effects](02-effects.md)) |
| `Nothing`, `Void` | Familiar from other languages | Read as absence, the confusion `Unit` already causes beside `none` |

The maintainer chose `Step`, as a subtype of a new type for the concept
itself, `Aute`, the Atomic UniT of Execution, said like *byte*. In the terms
of a declaration:

```text
Aute
Step : Aute
```

The pipeline occurrence and the type are one idea at two levels: running a
step, or a statement, is a unit of execution, and `Step` is what that unit
gives when it gives nothing else. `Aute` is the wider idea, any atomic
execution that completed, so that a later unit that is not a statement, a
transaction's attempt perhaps, has a place under it beside `Step`.

`Unit` named a unit of *computation*; `Aute` names a unit of *execution*, and
the difference is the point of the record. A computation is what the program
says; a pure one may be executed any number of times in any number of pieces,
and the program cannot tell. An execution is what happened once.

## Where `Unit` is used

About fifty files name it. By kind:

- **Statements with no value.** The checker gives `Unit` to a binding, a
  `type`, `fn`, `trait` or `impl` declaration and an `import`
  (`src/checker.rs`, the `Stmt` arms that return `TypeRef::UNIT`), and to a
  program whose last statement is one of these. `hql check` prints `Unit` for
  a file holding only `x = 1`.
- **Writes**: `target` and `sink` in `lib/std/io/mod.hql`, and the step form
  `sink(e, F)`, a `Stream<T> -> Unit`; `src/extensions/io.rs`.
- **Mutations**: `insert`, `update` and `delete` (HQL-0051).
- **Log calls**: `trace`, `debug`, `info`, `warn` and `error` in
  `lib/std/io/logging/mod.hql`; `src/extensions/logging.rs`.
- **The type itself**: `src/types/builtin.rs` (`UNIT`), `src/types/value.rs`
  (`Value::Unit`), `src/types/display.rs` (its printed and JSON forms).
- **Documentation**: about thirty mentions in the Learn pages and forty in the
  reference pages, and the `hql check` outputs the book records.

The Rust identifiers (`Value::Unit`, `TypeRef::UNIT`) MAY keep their names;
the rename is of the HQL type.

## Its value

Today the value is written `()`, prints `()` at the top level, an empty string
in some text forms, and JSON `null`; in a `fork` record it shows as `()`
beside `empty`. Issue 0072's value printing fix makes every value print one
way; `Step`'s value follows that rule. Whether it stays `()` or gets a name is
an open question of the record.

## The migration, in 0.6.0

1. `src/types/builtin.rs`: the builtin is named `Step`, and `Aute` is
   added as its supertype.
2. `Unit` where a type is expected is refused: "`Unit` is now `Step`".
3. Every `lib/` signature and doc string that says `Unit` says `Step`.
4. The Learn pages and the recorded `hql check` outputs; `hql reference`
   regenerates the reference pages' marked parts.
5. `CHANGELOG.md`, under "Changed", beside `length` to `size` and `socket` to
   `tcp`.

## Room for an outcome

When the backends report a count, an offset or a commit uniformly, a write
can give more than `Step`. A record type that subtypes `Step`,

```hql
type Written : Step { endpoint : String, count : Int }
```

would let `insert` give `Written` while a program checked against `Step`
still checks, giving the chain `Written : Step : Aute`. A `Result` would
instead let a failed write be routed. Which, and whether a streaming `sink`
reports progress as a value or only through the job, is issue 0063's
remaining question, not this record's.
