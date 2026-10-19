# HQL-0006: `Stream<T>` as a first-class pipeline type

**Status**: implemented
**Created**: 2026-09-28
**Source**: [queries](../../models/behavior/queries.md)

## Abstract

`Stream<T>` joins the core type lattice as the type of values produced in order
over time, with no promise that production ends. It is independent of
`Collection<T>`: a stream is ordered but not necessarily finite, a collection is
finite but not necessarily ordered, and `Seq<T>` is both. `map` and `filter`
gain stream signatures, `Stream<T> -> Stream<U>` and `Stream<T> -> Stream<T>`,
and every operation that needs the whole input — `size`, `sort`, `group`,
`reduce` — keeps requiring `Collection<T>` or `Seq<T>`, so that applying one to
an unbounded stream is a type error rather than a program that never returns.
A pipeline neither starts from nor ends in a materialized collection by
assumption.

## Motivation

- HQL pipelines are meant to describe long-running computations over event
  sources, not only queries over a vault that already sits in memory. Before
  this record every collection type in `std/collections.hql` was finite, so
  such a source had no type.
- [Queries](../../models/behavior/queries.md) already designs the hierarchy
  this needs — `Iterable`, `Collection`, `Stream`, `Seq`, and a lazy traversal
  carrier — but it was a proposed model, not a decision, and nothing in the
  core depended on it.
- [HQL-0007](../HQL-0007/README.md) (`collect` and `emit`) and
  [HQL-0004](../HQL-0004/README.md) (`group`, `reduce`) both need to say what
  they do to a stream. Without the type they can only be specified over
  collections, which is the one-MapReduce-cycle reading this work exists to
  leave behind.

## Goals

- A stream has a type, and the type says what may and may not be asked of it.
- An operation that would never finish on an unbounded input is refused by the
  checker.

## Non-goals

- **No live sources.** How an external event source binds into a program is an
  open question below. This record types streams; it does not connect one.
- **No pull.** `next` stays outside the design, as queries already argues.
- **No windowed aggregation.** What `group` means on a stream is left to its own
  record; see Open Questions.

## Specification

### The type

`Stream<T>` is an abstract type in the core.

```hql
// Values produced in a stable order; production need not end.
// No size, no membership, no global order: those need the end.
abstract type Stream<T> : Iterable<T>

abstract type Seq<T> : {Collection<T>, Stream<T>}
```

`Iterable<T>` is the floor [queries](../../models/behavior/queries.md)
describes: values that can be traversed, with no promise of finiteness, order
or uniqueness. Both `Collection<T>` and `Stream<T>` are `Iterable<T>`, and it is
the bound an operation states when it needs only to visit each value, as `map`
and `filter` do.

- A `Stream<T>` MUST guarantee a stable succession of values `x0, x1, x2, ...`.
- A `Stream<T>` MUST NOT be assumed finite. Neither its cardinality nor when
  production completes is part of the type.
- `Seq<T>` MUST become a subtype of `Stream<T>` as well as of `Collection<T>`.
  A finite ordered sequence is a stream that happens to end; nothing a stream
  promises is withheld by a sequence.
- `Set<T>` MUST NOT be a `Stream<T>`. Its enumeration order is not part of the
  value, and a stream's order is.
- `Stream<T>` is a semantic contract, not a runtime representation. The carrier
  is the restartable traversal described in
  [queries](../../models/behavior/queries.md), and it MUST NOT itself be named
  `Stream`.

### Stream is documented apart from collections

- `Stream<T>` MUST have its own wiki card beside the collection cards, not a
  paragraph inside them. The card states the contract above and the table of
  operations below.
- The collection cards MUST describe `Seq<T>` as the meeting point of the two
  families and link the stream card rather than restating it.

### Operations over a stream

Each operation declares the weakest input it needs and returns the type it
was given, in the form [issue 0018](../../issues/0018-operator-synonymy-audit.md)
settles: `S` binds the input's own type, `F<_>` its constructor.

```text
map<F<_> : Iterable<_>, T, U>(xs: F<T>, by: T -> U)  : F<U>
filter<S : Iterable<T>, T>(xs: S, by: T -> Bool)     : S
drop<S : Stream<T>, T>(xs: S, n: Int)                : S
take<S : Seq<T>, T>(xs: S, n: Int)                   : S
take<T>(xs: Stream<T>, n: Int)                       : List<T>
collect<T>(xs: Stream<T>, n: Int)                    : Stream<List<T>>    -- HQL-0007
emit<T>(xs: Stream<Seq<T>>)                          : Stream<T>          -- HQL-0007
```

- `map` MUST transform each value and MAY change its type.
- `filter` MUST decide which values continue and MUST preserve their type and
  relative order.
- `map` and `filter` MUST remain distinct operations. Filtering is not encoded
  as `map` to `Option<T>` followed by a flattening step, because that turns a
  type-preserving decision into a type change and back, and it would make
  `filter`'s guarantee — same element type — a property the checker has to
  rediscover.
- `map` over a `Stream<T>` MUST return a `Stream<U>`, over a `List<T>` a
  `List<U>`, and over a `Set<T>` its image, a `Set<U>`. `filter` MUST return
  its input's type.
- `drop` MUST skip the first `n` values and return its input's type: a stream
  without its first `n` values is still a stream that may not end, and a
  sequence is still a sequence. `drop` is the `OFFSET` of
  [issue 0016](../../issues/0016-sql-parity-analysis.md).
- `take` MUST accept a `Stream<T>` and return a `List<T>`, because a bounded
  prefix is finite whatever the input. Over a `Seq<T>` it keeps the input's
  type.

### Whole-input operations need a finite input

An operation that must see the end of its input before producing anything
MUST require `Collection<T>` or one of its subtypes.

```text
size, count, contains    : Collection<T>
sort                     : Collection<T>
group                    : Collection<T>      -- HQL-0004
reduce                   : Seq<T>             -- HQL-0004
List (materialization)   : Seq<T>             -- HQL-0007, HQL-0018
```

- Applying one of these to a `Stream<T>` that is not also a `Collection<T>`
  MUST be a type error. The message MUST name the two ways to bound the input:
  `take(n)` for a prefix and `collect(n)` for batches.
- This is the type-level form of [HQL-0004](../HQL-0004/README.md)'s barrier:
  every barrier operation requires a finite input, so the checker, not a
  timeout, rules out a barrier that can never close.

```hql
events | sort(by = e => e.at)                          // error: Stream is not finite
events | take(100) | sort(by = e => e.at)              // ok: List<Event>
events | collect(100) | map(b => sort(b, by = e => e.at))  // ok: Stream<List<Event>>
```

### A pipeline is typed step by step

- Every step MUST be checked against the type the previous step produced;
  there is no pipeline-level type.
- No step MAY assume that its input is materialized, and no pipeline is assumed
  to end in a collection. A pipeline over a `Stream<T>` that ends in a
  `Stream<U>` is complete as written; what consumes it is a presentation
  ([HQL-0008](../HQL-0008/README.md)) or the host.

## Backwards Compatibility

Additive at the type level. `Seq<T>` gains a supertype; no existing program
names `Stream`, and the rule that collection signatures win over a `Seq`
keeps every current result type. A user declaration named `Stream` is refused
as any redeclaration of a core type is.

## Security Considerations

An unbounded stream is unbounded work. `localhost` MUST NOT buffer a stream
except where an operation's contract says so (`collect(n)` buffers at most `n`
values). Resource bounds for live sources belong to the record that introduces
them.

## Reference Implementation

- `std/collections.hql` — `Stream<T>`, `Seq<T> : {Collection<T>, Stream<T>}`,
  and the stream signatures as comments beside the collection ones
- `src/types/` — `Stream` in the constructor table and the subtype relation
- `src/extensions/collections.rs` — stream overloads for `map`, `filter`,
  `take`, and `drop`; the finite-input diagnostic for whole-input operations
- `src/evaluation.rs` — a traversal carrier that does not force its input,
  shared with [issue 0010](../../issues/0010-yield-and-bind.md)
- `doc/wiki/hql/types/collections/` — a stream card, and `Seq<T>` described as
  the meeting point

## Test Plan

Unit tests MUST include:

- `Seq<Int>` is a subtype of `Stream<Int>`; `Set<Int>` is not
- `map` over `Stream<Int>` returns `Stream<U>`; over `List<Int>` it
  returns `List<U>`
- `drop(2)` over a `Stream<Int>` is a `Stream<Int>` and over a `List<Int>` a
  `List<Int>`; over an unbounded test source followed by `take(3)` it
  terminates
- `filter` over `Stream<T>` returns `Stream<T>` with order preserved
- `size`, `sort`, `group` and `reduce` over a `Stream<T>` are type errors whose
  message mentions `take` and `collect`
- `take(3)` over an unbounded test source terminates and returns three values,
  which shows `filter` and `map` upstream of it do not force the source

## Open Questions

- How does a live source bind? A traversal in
  [queries](../../models/behavior/queries.md) is a factory, so two traversals
  of one value see the same elements. An event source cannot promise that.
  Recommendation: a live source is an `Effectful` operation, and a pipeline
  over it runs once, where it is written, as [HQL-0004](../HQL-0004/README.md)
  already requires of effects.
- What does `group` mean on a stream? An unrestricted
  `events | group(by = e => e.user)` can never know that a group is complete.
  Grouping each batch is already expressible without a decision —
  `events | collect(100) | map(b => group(b, by = e => e.user))` — while
  `events | collect(100) | group(...)` as written groups the batches
  themselves. Whether `group` on a `Stream<T>` is refused, lifted per batch, or
  window-bound deserves its own record.
- Is there a way to forget finiteness explicitly, so that a finite test source
  can be typed `Stream<T>` in the corpus? A type ascription on a binding would
  do; if the grammar has none, a core `stream` operation that is the identity
  at runtime would.
- Is a running fold (`scan`) the stream counterpart of `reduce`, and does it
  belong to this family or to the windowing record?

## Changelog

- 2026-09-28: drafted
- 2026-10-19: implemented; `Stream<T>` and `Iterable<T>` are in the core, `map`,
  `filter`, `take` and `drop` take streams, and a whole-input operation over a
  stream is refused naming `take(n)` and `collect(n)`
