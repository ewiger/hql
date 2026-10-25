# HQL-0036: Partitioning, shuffle boundaries and execution scheduling

**Status:** proposed, implemented in part  
**Created:** 2026-10-23  
**Related:** HQL-0010 (`tee`), HQL-0033 (error routing), HQL-0037 (`fork` and the dataflow DAG)

## Abstract

`partition` changes the distribution of a collection or stream while preserving its element type.

Conceptually:

```hql
Stream<T>
| partition(by = x => key(x))
    : Stream<T>
```

The values remain `T`. The operation assigns values with the same partition key to the same logical partition so that downstream stateful operations may execute locally per partition.

`partition` is therefore different from:

- `tee`, which duplicates a logical flow into a side computation;
- `fork`, which broadcasts one flow to several first-class branches, such as `oks()` and `errs()` over `Result<T, E>` ([HQL-0037](../HQL-0037/README.md));
- ordinary `map`, `filter` and `flat_map`, which normally require no data redistribution.

A partition is a logical unit of data ownership and scheduling. It is not a worker, thread, process or Kafka partition, although a backend may map it onto those concepts.

---

## Motivation

Distributed and streaming execution eventually requires HQL to express a fundamental distinction:

```text
stateless local computation
        |
        v
redistribution by key
        |
        v
stateful local computation
```

For example:

```hql
orders
| map(o => {
    customer: o.customer_id,
    amount: o.amount
})
| partition(by = x => x.customer)
| reduce(...)
| sink(...)
```

The `map` may run independently on arbitrary input shards.

The `reduce` needs all values belonging to the same customer to meet at the same logical partition.

`partition` is the boundary that makes this requirement explicit.

---

## Semantics

For:

```hql
xs
| partition(by = x => f(x))
```

the following MUST hold:

1. Every input value appears exactly once in the partitioned output.
2. The value's type does not change.
3. Values with the same partition key belong to the same logical partition.
4. Partition identity is independent from physical worker identity.
5. Moving a logical partition between workers MUST NOT change query semantics.
6. A backend MAY implement the operation using hashing, range partitioning or another strategy consistent with the declared semantics.
7. `partition` MAY require network redistribution and therefore forms an execution-stage boundary.

Conceptually:

```text
Stream<T>
    |
 partition(by = key)
    |
    +---- partition 0 : T
    |
    +---- partition 1 : T
    |
    +---- partition 2 : T
```

The `T` is preserved.

This is intentionally different from:

```text
Stream<Result<T,E>>
    |
   fork
   /   \
oks()  errs()
  |      |
  T      E
```

where the branches are first-class results and have different types.

---

## Logical partitions are not workers

Given:

```text
64 logical partitions
8 workers
```

the executor may initially assign:

```text
worker 1 -> partitions 0..7
worker 2 -> partitions 8..15
...
```

and later rebalance them.

A partition therefore represents:

```text
data ownership
+ ordering scope
+ state locality
+ scheduling unit
```

not:

```text
one thread
one machine
one process
```

A worker may own several partitions, and a partition may move between workers.

---

## Execution stages

Stateless operators SHOULD normally be fused.

For:

```hql
source(users)
| map(parse)
| filter(valid)
| map(normalize)
| partition(by = u => u.account_id)
| reduce(...)
| sink(...)
```

an executor may compile:

```text
Stage A
────────────────────────────
source
parse
filter
normalize

        |
        | shuffle
        v

Stage B
────────────────────────────
partition-local reduce

        |
        v

sink
```

There is no semantic requirement to materialize intermediate values between `map`, `filter` and `map`.

`partition` is different because it may change ownership of the value and therefore normally ends the current fused execution stage.

---

## MapReduce

A conventional MapReduce plan is expressible directly:

```hql
events
| map(e => {
    key: e.country,
    count: 1
})
| partition(by = x => x.key)
| reduce(...)
```

The physical plan may be:

```text
parallel map
     |
local partial reduce
     |
   shuffle
     |
final partition-local reduce
```

If a reduction is associative and otherwise safe to combine, the executor MAY perform local partial reduction before the shuffle.

For example:

```text
worker A:
CH -> 12000
DE -> 8000

worker B:
CH -> 15000
FR -> 3000
```

may be shuffled instead of every original event.

The final reducer then produces:

```text
CH -> 27000
DE -> 8000
FR -> 3000
```

This is an execution optimization and MUST NOT change the result.

---

## Stateful streaming

Partitioning also provides the ownership boundary required for stateful streams.

For example:

```hql
transactions
| partition(by = t => t.account_id)
| map(update_account_state)
```

may establish:

```text
account A ──> partition 7 ──> state[A]
account B ──> partition 2 ──> state[B]
account C ──> partition 7 ──> state[C]
```

The executor may move partition 7 between workers, but the logical ownership and ordering guarantees attached to partition 7 remain.

This provides a foundation for future operations such as:

```text
reduce
aggregate
window
stateful map
keyed join
```

without exposing workers in HQL syntax.

---

## Partition count

The semantic form SHOULD remain:

```hql
partition(by = x => x.account_id)
```

Execution configuration determines physical parallelism.

A future hint may allow:

```hql
partition(
    by = x => x.account_id,
    partitions = 64
)
```

but partition count should be treated as an execution/layout hint unless a later ADR gives it semantic meaning.

Programs SHOULD NOT name workers or scheduler topology.

---

## Relation to other flow operations

```text
tee
    one logical value -> main flow + side computation
    input value continues unchanged

fork
    one flow -> several first-class branches
    every branch sees every input
    with oks() and errs(): Result<T,E> -> T branch + E branch

partition
    T -> partitioned T
    type is preserved
    values are redistributed by key

map/filter/flat_map
    normally local transformations
    normally fusible into one execution stage
```

`fork` is logical fan-out and is normally cheap; its branches are user-visible.

`partition` may imply serialization, buffering, network transfer, backpressure, checkpointing and state migration.

They MUST remain different language concepts.

---

## Backend model

A backend may compile HQL into a small number of graph primitives:

```text
Source
  |
Fused stateless stage
  |
Split / Tee
  |
Partition / Shuffle
  |
Stateful stage
  |
Sink
```

The scheduler is responsible for:

1. identifying stage boundaries;
2. assigning logical partitions to workers;
3. running fused stateless operations locally;
4. redistributing values when partition ownership changes;
5. scheduling stateful operators against partition-local state;
6. checkpointing state where required;
7. retrying failed execution tasks according to runtime policy;
8. committing sinks according to their delivery semantics.

These are backend concerns. HQL syntax describes the logical dataflow.

---

## Non-goals

This ADR does not define:

- a cluster scheduler;
- worker APIs;
- checkpoint storage;
- exactly-once delivery;
- Kafka-specific partition semantics;
- physical partitioning of files or database tables;
- join or window syntax;
- failure-recovery protocol.

Those may build on the logical partition abstraction defined here.

---

## Decision

Reserve `partition` for **same-type distribution by key and the corresponding execution boundary**.

Do not use `partition` for separating `Result<T, E>` into successful and failed values. That is a `fork` with `oks()` and `errs()`, as HQL-0033 and HQL-0037 specify. `partition` is logically `1 → 1`: its lanes are never first-class branches.

---

## The local backend

`partition(by = f)` is declared over `S : Seq<T>`, giving `S`, and over `Stream<T>`, giving `Stream<T>`, with the shape `partition`, so the planner opens a stage at it. The key `f` may give a value of any type. The local backend holds one logical partition per key on one worker: it computes each value's key, as a backend that redistributes would, and passes the values on in their input order, which keeps the order of each partition. Over a stream it does so one value at a time. The former predicate form, the values a predicate accepts and those it rejects as a pair, is not `partition`; it is a `fork` with complementary filters.

## Changelog

- 2026-10-23: proposed
- 2026-10-23: `partition(by = …)` built on the local backend
  ([issue 0045](../../issues/archive/0045-errors-validators-boundaries.md), P1), and
  the predicate `partition` removed; shown in the
  [dataflow examples](../../../examples/dataflow/README.md)
