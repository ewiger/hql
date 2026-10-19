# HQL-0007: `collect` and `emit` are stream-shape operations

**Status**: implemented
**Created**: 2026-09-28
**Source**: [typed pipeline operations](../../issues/0013-typed-pipeline-operations.md)

## Abstract

`collect(n)` batches a stream: it takes a `Stream<T>` and produces a
`Stream<List<T>>` whose batches hold consecutive values, `n` to a batch, the
last possibly fewer. It does not end a pipeline and does not materialize its
input. `emit` is its inverse: it takes a `Stream<Seq<T>>` and produces each
member of each batch, in order, as a value of a `Stream<T>`. The argument list
of `collect` is shaped as a window so that time bounds (`for = 5s`) can be
added later without changing the step node. The name `collect` stops meaning
materialization; a finite sequence becomes a `List` by the `List`
constructor, `xs | List`, as [HQL-0016](../HQL-0016/README.md) specifies.
`flat_map(f)` is the shorthand for `map(f) | emit`.

## Motivation

- [HQL-0004](../HQL-0004/README.md) specifies `collect : Seq<T> -> List<T>` as
  a materialization barrier and, in its Open Questions, asks whether that step
  should be `collect` or `list`. A barrier over an unbounded stream never
  closes, so under [HQL-0006](../HQL-0006/README.md) that `collect` could only
  ever appear at the end of a finite pipeline.
- Long pipelines need a batch boundary in the middle: accumulate some values,
  compute over the batch (sort it, group it, score it together), and continue
  streaming. Nothing in the core expresses that.
- A batch that goes in must be able to come back out. Without an inverse, every
  pipeline that batches once is a pipeline of batches forever after.

## Goals

- `collect` introduces batching anywhere in a pipeline, including over an
  unbounded stream.
- `emit` undoes it, so that `xs | collect(n) | emit` is `xs`.
- The representation leaves room for count-and-time windows.

## Non-goals

- **No time windows yet.** `for = 5s` is reserved, not implemented; see the
  heading on reserved time windows below.
- **No keyed windows.** Batching by key is the grouping question
  [HQL-0006](../HQL-0006/README.md) leaves open.
- **No `flatten`.** See The name is `emit`.

## Specification

### `collect(n)`

```text
collect<T>(xs: Stream<T>, n: Int) : Stream<List<T>>
collect<T>(xs: Seq<T>,    n: Int) : List<List<T>>
```

- `n` MUST be a positive `Int`; `collect(0)` and a negative `n` MUST be
  refused, by the checker when `n` is a literal and at run time otherwise.
- Batch `i` MUST hold the values at positions `i*n` through `i*n + n - 1` of
  the input, in order. Every batch except the last MUST hold exactly `n`
  values; the last holds between `1` and `n`. An empty input produces no
  batches, never one empty batch.
- `collect` MUST NOT terminate the pipeline. Its output is a stream, and every
  operation that accepts a stream may follow it.
- `collect` MUST NOT buffer more than `n` values at once. It emits each batch
  as soon as the batch is full, and the last one when its input ends.
- Over a `Seq<T>` the result is finite and MUST be a `List<List<T>>`, by the
  rule in [HQL-0006](../HQL-0006/README.md) that an operation preserves every
  guarantee it can prove.
- The batch type is `List<T>` rather than `Seq<T>`: a batch has been buffered
  in full before it is emitted, so it is materialized, and saying so lets a
  batch be passed to anything that needs a `List`. `List<T>` is a `Seq<T>`, so
  `emit` accepts it.
- Batching by position is deterministic, so every backend MUST produce the same
  batches as `localhost`.

```text
[a, b, c, d, e] | collect(2)   =  [[a, b], [c, d], [e]]
```

### `emit`

```text
emit<T>(xs: Stream<Seq<T>>) : Stream<T>
emit<T>(xs: Seq<Seq<T>>)    : List<T>
```

- `emit` MUST produce every member of every input sequence, members of one
  sequence in their positions, sequences in their order.
- An empty inner sequence contributes nothing.
- For every stream `xs` and positive `n`,
  `xs | collect(n) | emit` MUST equal `xs`.
- `emit` MUST accept only sequences as elements: every batch is a `Seq<T>`. A
  `Stream<Set<T>>` is refused, with a message saying to sort the sets first, because a set's members have no
  order and emitting them would invent one. Generalizing the element to
  `Collection<T>` is an open question.

### The name is `emit`

`flatten` names a structural transformation of a nested collection. `emit`
names what happens to the downstream stream: the members of each batch enter it
as values. The operation is the second thing, and the name says so. A program
that writes `flatten` MUST get an unknown-operation diagnostic that suggests
`emit`.

### `flat_map` is the shorthand for `map` then `emit`

```text
flat_map<T, U>(xs: Stream<T>, f: T -> Seq<U>) : Stream<U>
flat_map<T, U>(xs: Seq<T>,    f: T -> Seq<U>) : List<U>

xs | flat_map(f)  =  xs | map(f) | emit
```

- `flat_map(f)` MUST equal `map(f) | emit` over every input, in values and in
  order. The longhand is written in HQL, which is what admits the shorthand.
- `f` MUST return a sequence, by the same rule as `emit`: a function returning a
  `Set` is refused with a message saying to sort it.
- `flatten` is not an alias of `flat_map` or of either signature of `emit`.

### Materialization is the `List` constructor

The operation [HQL-0004](../HQL-0004/README.md) called `collect` is the
constructor of `List`, as [HQL-0016](../HQL-0016/README.md) specifies it.

```text
List<T>(xs: Seq<T>) : List<T>
```

- `xs | List` is `List(xs)`. It returns the sequence's values at their
  positions; over a `List<T>` it is the identity.
- It refuses a `Stream<T>` that is not a `Seq<T>`, naming `take(n)`, and a
  `Set<T>`, naming `sort`.
- This resolves the HQL-0004 open question "Is the materialization step
  `collect` or `list`?": neither; the step is `List`.

### Shape

The operations join the table in [HQL-0004](../HQL-0004/README.md).

| Operation | Purity | Shape | Input | Output |
| --- | --- | --- | --- | --- |
| `collect(n)` | pure | batch | `Stream<T>` | `Stream<List<T>>` |
| `emit` | pure | elementwise | `Stream<Seq<T>>` | `Stream<T>` |
| `flat_map(f)` | pure | elementwise | `Stream<T>` | `Stream<U>` |
| `List` | pure | barrier | `Seq<T>` | `List<T>` |

- **Batch** is a new shape. A batch step needs `n` consecutive values, not the
  whole input, before it produces output. A planner MAY split its input only on
  batch boundaries, which is why the boundaries are positional.
- `emit` is elementwise: the output for one batch depends on that batch alone,
  and outputs are concatenated in input order.

### Time windows are reserved

The API leaves room for three forms; only the first is implemented.

```hql
| collect(100)
| collect(for = 5s)
| collect(100, for = 5s)
```

- The checked argument of `collect` MUST be a window value with an optional
  count and an optional duration, not a bare `Int`, so that adding the duration
  changes a field and not the execution tree.
- At least one of the two MUST be present. When both are, a batch closes at
  whichever bound is reached first.
- Until durations exist, the duration field is always absent, and writing the
  named argument MUST be an error that says time windows are not implemented
  yet and names `collect(n)`.

## Backwards Compatibility

No `examples/mapreduce/` case used `collect` as materialization.
[HQL-0004](../HQL-0004/README.md) and
[map/reduce for queries](../../models/behavior/map-reduce.md) write `List` for
it. `emit` and `flat_map` are new names, read only after `|` or
called directly, so they neither collide with nor shadow a binding.

## Security Considerations

`collect(n)` holds at most `n` values; a program chooses `n`, so a very large
literal is a very large buffer. The checker SHOULD warn when a literal `n`
exceeds a host limit. The limit is an open question.

## Reference Implementation

- `src/extensions/collections.rs` — `collect`, `emit`, `flat_map`, and the
  reserved `for` argument; the `Batch` shape arrives with HQL-0004's shape
  metadata
- `std/collections.hql` — the signature comments
- `examples/pipelines/` — the batching queries listed in
  [issue 0013](../../issues/0013-typed-pipeline-operations.md)

## Test Plan

Unit tests MUST include:

- `[1, 2, 3, 4, 5] | collect(2)` is `[[1, 2], [3, 4], [5]]` at
  `List<List<Int>>`
- `[] | collect(3)` is `[]`
- `collect(0)` is a check error
- `xs | collect(n) | emit` equals `xs` for several `xs` and `n`
- `emit` over a stream of sets is a type error mentioning `sort`
- `xs | flat_map(f)` equals `xs | map(f) | emit` over a list and over an
  unbounded test source
- `flatten` is an unknown operation whose diagnostic suggests `emit`
- over an unbounded test source, `collect(3) | take(2)` terminates with two
  batches of three
- `collect(for = 5s)` is refused with the not-implemented message

## Open Questions

- Should `emit` generalize to `Stream<Collection<T>>`? It would have to drop to
  an unordered result type, `Iterable<T>`, for set elements.
- What is the named argument for a duration? `for` reads well but is the
  iteration keyword in [queries](../../models/behavior/queries.md); `within` and
  `every` avoid the collision.
- A time window depends on a clock, so its batches are not a function of the
  input alone. Is a time-windowed `collect` `Effectful`, or is the clock an
  input the backend supplies?
- What host limit on `n` triggers the warning, and is it configurable?

## Changelog

- 2026-09-28: drafted
- 2026-10-18: `flat_map` added as the shorthand for `map(f) | emit`
- 2026-10-18: materialization is the `List` constructor (HQL-0016); `list`
  retired
- 2026-10-19: implemented; `collect(n)`, `emit` over sequences, `flat_map` and
  materialization by `List` are built, `collect(for = …)` is refused as not
  implemented yet, and the `Batch` shape waits on HQL-0004's shape metadata
