# HQL-0050: Analytic windows, and a deterministic order for keyed selection

**Status**: drafted
**Created**: 2026-11-12
**Source**: [Deduplication](../../public/learn/20-olap/deduplication.md),
[HQL-0014](../HQL-0014/README.md) (`reduce` and `merge` as a combiner),
[HQL-0036](../HQL-0036/README.md) (partitioning and stateful streaming),
[HQL-0044](../HQL-0044/README.md) (type checking is the semantic boundary),
`lib/std/collections/mod.hql`, `lib/std/seq/mod.hql`, `lib/std/core/mod.hql`

## Abstract

`over` is a new sequence operation: an analytic window. It partitions a
`Seq<T>` by a key, orders each partition, and maps every row to a value
computed from the row and its `Window<T>`, the row's place in its ordered
partition. The result has one value per input row, at the row's input
position; no row is merged or dropped. `Window<T>` gives `row_number`,
`rank`, `dense_rank`, `lag`, `lead`, `first`, `last` and three frames:
`running`, `rows(before, after)` and `whole`. Two existing rules are made
exact so that keyed selection is deterministic on every backend: equal
elements under an ordering are resolved by input position, and a backend
that splits a `Seq` carries that position. `Ordering` gains `then`, which
composes comparisons into a total order. The keyed selection
`group(by = k) | map(g => g.items | max_by(o)) | somes` and its `over`
form are one logical operation that a backend MAY execute as a combinable
reduction without building `Group.items`.

## Motivation

- `group` with `max_by` is a keyed argmax: it keeps one row per key. It
  does not express what SQL's analytic functions express: the three latest
  orders per customer, a running balance, the difference from the previous
  revision, a row's rank among its peers. Each of these keeps every row and
  adds a value computed from the row's neighbours.
- The three operations HQL has that sound alike are something else.
  `group` merges rows into groups. `partition` is a physical stage
  boundary ([HQL-0036](../HQL-0036/README.md)) and computes nothing.
  `window(n)` is the runs of `n` consecutive elements of one sequence.
- Deduplication by key is correct only under a total order. Two different
  rows with one `id` and one `at` are a tie, and `max_by` keeps the first by
  input position. The local backend keeps input position; a backend that
  splits the input has no rule yet for what "first" is, so the same program
  could keep a different row.
- `group` materializes every occurrence of a key as `Group.items` to keep
  one of them. Over a large table that cost is the whole input held at once,
  where a partial reduction per split holds one candidate per key.

## Goals

- One operation for analytic windows, typed, with the window functions of
  SQL's `OVER` clause as named operations on a value of type `Window<T>`.
- The same result for a program on every backend, ties included.
- Keyed selection and ranked selection executable without materializing
  groups.

## Non-goals

- Deduplication across micro-batches of a stream. That is keyed state with
  retention, the stateful streaming of [HQL-0036](../HQL-0036/README.md),
  or the merge write mode the [Merge](../../public/learn/20-olap/merge.md)
  page describes.
- A timestamp type. Ordering by an ISO 8601 `String` is correct only where
  every value is UTC in one format; the time type is
  [HQL-0041](../HQL-0041/README.md)'s.
- Range frames over values (`RANGE BETWEEN INTERVAL ...`). Frames are
  counted in rows.
- Executing `over` as SQL on a Spark endpoint. Evaluation stays in `hql`,
  as [HQL-0049](../HQL-0049/README.md) states; a pushdown is a later
  proposal.
- A dedicated deduplication operation. `distinct`, keyed selection and
  `over` cover the cases.

## Specification

### 1. Input position and ties

Every ordering-dependent operation resolves equal elements the same way.

- Over a `Seq<T>`, an element's input position is its index in that
  sequence. `max_by`, `min_by`, `sort`, `distinct` and `over` MUST resolve
  elements that compare equal by input position: the earlier one is first.
  This is the existing rule of `max_by`, `min_by` and `distinct`, and makes
  `sort` stable.
- A backend that splits a `Seq` across partitions or workers MUST carry
  each element's input position, as the pair of the split's ordinal and the
  element's offset within the split, and MUST resolve ties by it. The result
  of a program is then independent of how it was split.
- `Group.items` is in input order, as now. A `Set` has no input position;
  an ordering-dependent operation over a `Set` resolves ties by the set's
  own order.

The input position of a `source` read is the order the endpoint returns
rows in. A table without a declared order does not return rows in a stable
order across reads, so a program that deduplicates such a table SHOULD
state a total order (section "A total order") rather than rely on position.

### 2. A total order

`then` composes two comparisons, the second deciding where the first is
`Equal`.

```hql
/// `self` where it is `Less` or `Greater`, otherwise `next`:
/// `compare(1, 1) | then(compare("a", "b"))` is `Less`.
fn then(self: Ordering, next: Ordering) : Ordering
```

- `then` is declared in `core`, beside `Ordering`.
- A comparison written with `then` over a key that identifies a row, such
  as a source's change sequence number, is a total order, and keyed
  selection under it has no ties.

Keeping the latest revision of each order, with the change sequence
deciding between two revisions written in one second:

```hql
rows
| group(by = r => r.id)
| map(g => g.items | max_by((a, b) => compare(a.at, b.at) | then(compare(a.seq, b.seq))))
| somes
```

### 3. `over`

`over` is declared in `seq`:

```hql
/// Each value of `self`, at its input position, as what `into` gives for
/// it and its window: the values with its key `by`, ordered by `order`.
fn over<T, K, O : Orderable, U>(self: Seq<T>, by: T -> K, order: T -> O, into: (T, Window<T>) -> U) : List<U>
fn over<T, K, U>(self: Seq<T>, by: T -> K, order: (T, T) -> Ordering, into: (T, Window<T>) -> U) : List<U>
fn over<T, O : Orderable, U>(self: Seq<T>, order: T -> O, into: (T, Window<T>) -> U) : List<U>
fn over<T, U>(self: Seq<T>, order: (T, T) -> Ordering, into: (T, Window<T>) -> U) : List<U>
```

- `by` is the partition key, as in `group`. Without `by`, the whole input
  is one partition.
- `order` is the order within a partition: a key, ascending, or a
  comparison, as in `sort`. Ties are resolved by input position
  (section "Input position and ties").
- The result has exactly one value per input element, at that element's
  input position. `over` neither merges nor drops rows; `filter` after it
  does.
- `into` MUST NOT give a value whose type contains `Window<T>`. The checker
  refuses such a program. A window is a view of the partition during the
  call; letting it escape would force every backend to keep the partition.
- `over` takes a `Seq`. A `Set` is ordered first with `sort`. A `Stream`
  is refused, for the reason `group` refuses one: a partition of a stream
  that does not end is never complete. Inside `collect(n)`, `over` applies
  to each batch.

### 4. `Window<T>`

`Window<T>` is the current row's place in its ordered partition. Positions
are 1-based and in the partition's order.

```hql
type Window<T>

fn row_number(self: Window<T>) : Int
fn rank(self: Window<T>) : Int
fn dense_rank(self: Window<T>) : Int
fn size(self: Window<T>) : Int
fn lag(self: Window<T>, n: Int = 1) : Option<T>
fn lead(self: Window<T>, n: Int = 1) : Option<T>
fn first(self: Window<T>) : T
fn last(self: Window<T>) : T
fn running(self: Window<T>) : List<T>
fn rows(self: Window<T>, before: Int, after: Int) : List<T>
fn whole(self: Window<T>) : List<T>
```

- `row_number` is the row's position. Two rows never share one.
- `rank` is one more than the number of rows strictly before the row by
  `order`; rows equal by `order` share it, and the next distinct row skips
  the shared positions. `dense_rank` does not skip.
- `size` is the number of rows in the partition. `count` is its alias, as
  for every collection.
- `lag(n)` is the row `n` positions before, and `lead(n)` the row `n`
  after; `none` past either end. An `n` below 0 is an evaluation error.
- `first` and `last` are the partition's first and last rows.
- `running` is the rows from the first up to and including the current row,
  by `row_number`. This is SQL's `ROWS BETWEEN UNBOUNDED PRECEDING AND
  CURRENT ROW`, not SQL's default frame, which includes the current row's
  peers. `rows(before, after)` is the rows from `before` positions before
  to `after` positions after, clipped to the partition. `whole` is the
  partition in order.
- Aggregates over a frame are ordinary pipeline operations:
  `w.running | map(t => t.amount) | sum`.

### 5. The cases `over` serves

The latest revision of each order, as ranked selection:

```hql
rows
| over(by = r => r.id,
       order = (a, b) => compare(b.at, a.at) | then(compare(b.seq, a.seq)),
       into = (r, w) => {row: r, n: w.row_number})
| filter(x => x.n == 1)
| map(x => x.row)
```

The three latest orders of each customer:

```hql
orders
| over(by = o => o.customer, order = (a, b) => compare(b.at, a.at),
       into = (o, w) => {order: o, n: w.row_number})
| filter(x => x.n <= 3)
| map(x => x.order)
```

A running balance per account:

```hql
ledger
| over(by = t => t.account, order = t => t.seq,
       into = (t, w) => {account: t.account, seq: t.seq,
                         balance: w.running | map(x => x.amount) | sum})
```

The change from the previous revision:

```hql
rows
| over(by = r => r.id, order = r => r.seq,
       into = (r, w) => {id: r.id, at: r.at,
                         delta: w.lag | map(p => r.amount - p.amount)})
```

### 6. Keyed selection as one operation

Selecting one row per key is one logical operation with two spellings.

- These programs denote the same value up to output order:

  ```hql
  xs | group(by = k) | map(g => g.items | max_by(o)) | somes
  xs | over(by = k, order = o', into = (x, w) => {x: x, n: w.row_number})
     | filter(r => r.n == 1) | map(r => r.x)
  ```

  where `o'` is `o` reversed. `min_by` corresponds to `o` itself.
- A backend MAY execute either as a combinable reduction per key: each
  split keeps one candidate per key, and candidates are combined under the
  order, with input position deciding ties (section "Input position and
  ties"). That reduction is associative, so it is a partial reduce of
  [HQL-0036](../HQL-0036/README.md) and needs no `Group.items`.
- The same holds for `row_number <= n` with a constant `n`, kept as a
  per-key bounded top-`n`.
- The local backend MAY materialize groups. Recognising the pattern is an
  execution strategy and does not change a program's value
  ([HQL-0044](../HQL-0044/README.md)).

## Backwards Compatibility

The change is additive. `group`, `max_by`, `min_by`, `partition` and
`window(n)` keep their signatures and their values on the local backend.
`sort` is now specified as stable; the local implementation is stable
already. `then` and `over` are new names in `core` and `seq`.

## Security Considerations

`over` holds one partition at a time while it evaluates `into` on the local
backend, the bound `group` has. A frame is a view of that partition, not a
copy; `whole` and `running` called on every row of a partition of `n` rows
are `O(n^2)` work when aggregated, and the reference page states this. The
keyed-selection reduction holds one candidate per key per split.

## Deployment / Activation

1. Add `then` to `core`, and state the tie rule in the doc strings of
   `sort`, `max_by`, `min_by` and `distinct`.
2. Add `Window<T>` and `over` to `seq` with the local implementation and
   the check that refuses `Window<T>` in the result of `into`.
3. Write the example corpus below as `status: proposed`, and promote each
   case as it checks.
4. Update the Deduplication page: a total order with `then`, and `over` as
   the SQL `ROW_NUMBER` form beside `max_by`.
5. Add the keyed-selection reduction to the plan of a splitting backend
   when such a backend exists.

## Reference Implementation

- `lib/std/core/mod.hql`: `then`.
- `lib/std/seq/mod.hql`: `Window<T>`, its operations, and `over`.
- `src/extensions/seq.rs`: `over` and the window operations.
- `src/extensions/binding.rs`: binding of the new declarations.
- `src/backends/plan.rs`: input position carried through splits, and the
  keyed-selection reduction.
- The checker: refusal of `Window<T>` in `into`'s result, and of `over`
  over a `Stream`.

## Test Plan

Example cases under `examples/pipelines/queries/`, each with a
`proposal: doc/proposals/HQL-0050/README.md` header:

- Ties: two rows of `id = 6` with equal `at` and amounts 15 and 18 keep 15;
  reversing the input keeps 18; adding `then(compare(a.seq, b.seq))` keeps
  the row with the greater `seq` in both orders.
- `over` keeps input position: the result of an `into` that returns the
  row itself equals the input.
- `row_number`, `rank` and `dense_rank` over a partition with a tie:
  `[1, 2, 3]`, `[1, 1, 3]`, `[1, 1, 2]`.
- `lag` and `lead` at both ends give `none`.
- `running` sum over a three-row ledger gives the running balances.
- Keyed selection in both spellings gives the same set over the shop
  vault's `silver_replayed`, and the sum 188.49 after filtering placed
  orders.
- Refusals: `over` over a `Stream`, and an `into` that returns `w`.

Unit tests MUST include a split execution of keyed selection whose result
equals the unsplit one when the tied rows fall in different splits.

```bash
cargo build --locked
cargo test
```

## Changelog

- 2026-11-12: drafted
