# HQL-0026: Positions and slices: `xs[i]`, `xs[a:b]` and the `Slice` capability

**Status**: implemented
**Created**: 2026-10-22
**Source**: [the slicing example corpus](../../../examples/slicing/README.md)

## Abstract

The bracket after a value is one operator whose index decides its meaning: an
`Int` reads a position, and a `Range` takes a slice. `a:b` inside the bracket
builds a `Range`, either bound may be left out, and a negative position counts
from the end, for a position and a slice alike. A slice is of the type it is
taken from and never fails: a bound past the end is the end, and a reversed
range is empty. Slicing resolves through a core capability, `Slice`, which
String, the finite sequences and Data implement natively and a declared type
implements for its own values. A String becomes indexed, as a sequence of
one-character Strings. A Stream takes a slice by its bounds: a bounded slice
is a List, an open one stays a Stream.

## Motivation

- `xs[i]` exists for sequences and Data, but there is no way to write "a part
  of" one: lists compose `drop` and `take`, and strings call `slice(start,
  end)`, two spellings for one idea.
- The last element, or the last few, has no direct form: `xs[-1]` is absence
  today, and the end of a sequence is reached only through its length.
- A String has positions but is not indexed, although `char_at` and `slice`
  already treat it as a sequence of characters.
- Operators resolve through capabilities ([HQL-0024, operator
  contracts](../HQL-0024/parts/06-operators.md)); a slice over a declared type
  needs the same route rather than a list of built-in cases in the checker.

## Non-goals

- A step, `xs[a:b:s]`. Every other element is `enumerate` with `filter`.
- Assignment through a bracket. Values are immutable.
- `a:b` as an expression outside a bracket. A `Range` elsewhere is built with
  its constructor.

## Specification

### 1. Grammar

The bracket keeps its current attachment rule and gains a range form.

```text
index  = postfix "[" ( expression | range ) "]"
range  = [ expression ] ":" [ expression ]
```

- A bracket MUST touch the value it indexes; one after a space or a line break
  opens a list, as today.
- `xs[:]`, `xs[a:]`, `xs[:b]` and `xs[a:b]` are all ranges; `xs[]` is a syntax
  error.
- The receiver is evaluated first, then the start bound, then the end bound.

### 2. The `Range` type

`a:b` is a value of a core type, so a window named once is reused.

```hql
type Range { start? : Int, end? : Int }
```

- `xs[a:b]` MUST mean the same as `xs[Range(start = a, end = b)]`; a missing
  bound is `none`.
- `Range` is declared in [`std/core.hql`](../../../std/core.hql) and is in
  scope in every program.
- A bracket whose index is neither `Int` nor `Range` is a type error: "an
  index is an Int position or a Range, not {T}".

### 3. Resolving a position

Positions count from 0 over a length `n`; a negative position counts from the
end.

```text
resolve(p, n) = p + n   if p < 0
              = p       otherwise
```

- `xs[i]` MUST be the element at `resolve(i, n)` when `0 <= resolve(i, n) < n`,
  and absence otherwise. The result type is `Option<T>`, as today.
- A slice MUST clamp each resolved bound into `[0, n]`: a missing start is 0,
  a missing end is `n`. With `lo` and `hi` the clamped bounds, the slice is
  the elements at `lo` up to, not including, `hi`, and empty when
  `lo >= hi`.
- A slice MUST NOT fail on any `Int` bounds over a finite receiver. A
  position past the end is absence, so a part past the end is shorter, not an
  error.

### 4. What each receiver gives

A slice is of its receiver's type; a position is an `Option` of its element.

| Receiver | `xs[i]` | `xs[a:b]` |
| --- | --- | --- |
| `String` | `Option<String>`, one character | `String` |
| `List<T>`, `Seq<T>` | `Option<T>` | the receiver's type |
| `Ranking<T>` | `Option<Hit<T>>` | `Ranking<T>` |
| `Data` | `Option<Data>` | `Data` |
| `Option<S>` | `Option` of `S`'s position | `Option<S>`; absence stays absence |
| `Stream<T>` | refused, as today | see the Stream rules below |
| `Set<T>`, `Map<K, V>` | refused, as today | refused, with the same messages |
| a declared type | `Option<T>` through its `Index<T>`, refused without one | the type itself, through its `Slice`, refused without one |

- A String's positions MUST count characters, as `length`, `char_at` and
  `slice` do.
- Data holding something other than a sequence is a runtime error naming its
  shape, as `xs[i]` is today.

### 5. Streams

A Stream may not end, so a slice of one is decided by its bounds.

- `xs[:b]` MUST be `take(b)`, and `xs[a:b]` MUST be `drop(a) | take(b - a)`:
  both end, so both are `List<T>`.
- `xs[a:]` MUST be `drop(a)` and `xs[:]` the Stream itself: both are
  `Stream<T>`.
- A negative bound MUST be refused, because counting from the end needs the
  end: a literal one at typecheck, "{T} may not end, so a position counted
  from its end is not known", and a computed one at evaluation with the same
  message.
- Evaluation stays lazy: a bounded slice consumes at most `b` values.
- A Stream is sliced only with a written `a:b`; a `Range` value in the
  bracket over a Stream is a type error, since its type does not show whether
  the end is present.

### 6. The `Slice` capability

`xs[a:b]` resolves the slicing capability applicable to the receiver's type.

```hql
trait Slice {
    fn slice(self, range: Range) : Self
}
```

- `Slice` is a language-owned core trait in
  [`std/core.hql`](../../../std/core.hql), beside the operator traits.
- `String`, `Seq` and `Data` implement it natively with bodyless headers,
  `impl Slice for String` and `impl<T> Slice for Seq<T>`, the generic header
  of [HQL-0027](../HQL-0027/README.md); a List and a Ranking take `Seq`'s.
  Binding holds the headers and the native bodies to each other, as for the
  other core traits.
- A position resolves through `Index<T>`, the core trait of HQL-0027:
  `xs[i]` over a declared type implementing it is `xs.at(i)`, and the rules
  for [resolving a position](#3-resolving-a-position) bind the native
  implementations.
- A declared type implements `Slice` for its own values; the rules for
  [resolving a position](#3-resolving-a-position) bind the built-in implementations, and
  a declared one says what a part of its value is.
- `Stream` does not implement `Slice`: its slice is not of its own type, and
  the [Stream rules](#5-streams) are language-defined.
- The String receiver module keeps `slice(self, start: Int, end: Int)`, which
  MUST mean `self[start:end]`. A call names one of the two by its argument
  count.
- A receiver with no slicing is a type error: "{T} is not sliced; a sequence,
  a String, Data or a type implementing Slice is".

## Backwards Compatibility

- `xs[-1]` changes from absence to the last element, and every negative
  position likewise. No example or test relies on the old behavior.
- `s.slice(start, end)` keeps its meaning, and now also counts a negative
  bound from the end.
- `xs[i]` over a String was a type error and now checks.
- Nothing else that parses today changes meaning: `:` inside a touching
  bracket was a syntax error.

## Reference Implementation

- `src/parser.rs`, `postfix`: the range form inside a touching bracket.
- `src/ast.rs`: `Kind::Slice(receiver, start, end)` beside `Kind::Index`.
- `std/core.hql`: `Range`, `trait Slice`, and the native headers.
- `src/checker.rs`: `indexed` extended to String and to negative positions;
  a `sliced` counterpart giving [what each receiver gives](#4-what-each-receiver-gives),
  with the Stream rules and trait dispatch for declared types.
- `src/evaluation.rs`: `at_position` resolving negative positions, and the
  clamped slice over text, elements and Data; Stream slices built on the
  existing `take` and `drop` traversals.
- `src/extensions/string.rs`: `eval_slice` counting negative bounds from the
  end.
- A wiki card for the bracket operator under `doc/wiki/hql/`, and the
  [indexing example](../../../examples/birds/queries/indexing.hql) gaining a negative
  position.

## Test Plan

The [slicing corpus](../../../examples/slicing/README.md) pins every valid
case as `status: proposed`; each is promoted to `valid-now` as it checks.

Invalid cases to add with the implementation, each with its stage and error:

- a slice of a Set, `TypeMismatch` at typecheck
- a slice of a Map, `TypeMismatch` at typecheck
- `naturals(1)[-3:]`, `TypeMismatch` at typecheck
- a Stream sliced with a computed negative bound, `RuntimeError` at evaluate
- `xs[1.5]` and `xs["a":2]`, `TypeMismatch` at typecheck
- a declared type without `Slice` in a range bracket, `TypeMismatch` at
  typecheck
- `xs[]`, `SyntaxError` at parse

```bash
cargo build --locked
cargo test --locked --test corpus
```

## Changelog

- 2026-10-22: drafted
- 2026-10-22: implemented; `Range` is declared with `start? : Int` and
  `end? : Int`, and a Stream is sliced only with a written `a:b`
  ([issue 0035](../../issues/0035-positions-and-slices.md))
