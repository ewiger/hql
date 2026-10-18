# HQL-0013: Joins over named sides

- **Status**: drafted
- **Created**: 2026-10-18
- **Source**: [SQL parity analysis](../../issues/0016-sql-parity-analysis.md)

## Abstract

`join` combines sequences, in three forms of one operation told apart by their
input and argument types. The **binary form** joins the pipeline value to one
other sequence, as LINQ's `Join` does:
`birds | join(sightings, left = b => b.name, right = s => s.species, into = (b, s) => …)`.
Key functions `left` and `right` state an equality join, `on` states any other
condition, and `into` builds each result from a matching pair, so the binary
form needs no new type. The **head form** starts a query from many sources,
written as a record of named **sides** piped into `join`:
`{b: birds, s: sightings} | join(on = r => r.b.name == r.s.species)` yields
**rows** of type `{b: Card, s: Data}`. The **extending form** adds sides to
rows already under way: `rows | join({c: countries}, on = …)`. Outer joins are
`left_join`, `right_join` and `full_join` in the binary form, and a side marked
`?`, `{b: birds, s?: sightings}`, in the head and extending forms; a missing
side has type `Option<T>`, never `NULL`. Results are ordered by the positions
of their sides, so every join is deterministic. `cross_join` is the only join
without a condition, and `semi_join` and `anti_join` filter a sequence by
whether a match exists. Which modes a join admits follows from its sides'
types: finite sides admit every mode; a stream side may be joined to finite
sides it is matched against, but not to another stream, and never in a mode
that must wait for the stream's end; a stream made finite with `take` and
`list` joins as a batch.

## Motivation

- [Issue 0016](../../issues/0016-sql-parity-analysis.md) ranks joins among the
  largest gaps against SQL. `INNER JOIN` and `CROSS JOIN` are nested-`map`
  workarounds today that produce nested results, and outer joins have no
  spelling at all.
- [HQL-0012](../HQL-0012/README.md) makes a vault, a table and a topic peers.
  Peers that cannot be combined are not much use: the first question asked of a
  database beside a vault is which cards mention which rows.
- A query that starts from many sources has no head today. Every pipeline begins
  with one value, and there is no way to say "these three, together" before the
  first `|`.
- [HQL-0010](../HQL-0010/README.md) branches a pipeline and leaves recombining
  branches to an explicit operation. `join` is that operation.

## Goals

- a binary join over the pipeline value that needs no new type
- one `join` that starts a query from many sources and continues one already
  under way
- inner, left, right, full outer, cross, semi and anti joins, without `NULL`
- equality keys stated directly, and recognized in a predicate, so a planner
  can hash a join or push it into a source
- deterministic result order, so `take` after a join answers the same
  everywhere

## Non-goals

- **No set operations.** `union`, `intersect` and `except` need collection
  equality, which issue 0016 leaves to its own step.
- **No stream-to-stream joins.** Joining two unbounded streams needs time
  windows, which [HQL-0007](../HQL-0007/README.md) reserves. A stream joined to
  a finite side is in scope.
- **No general record values.** Rows are the one structural record the head and
  extending forms need; constructing declared records is issue 0016's
  roadblock 5.
- **No comprehension syntax.** A `for … in … where … yield` form may later
  spell the same join; it would be sugar over this one.

## Specification

### 1. Terms

- **binary form**: a join of the pipeline value with one other sequence, whose
  result is built by `into`.
- **side**: one named input of a head or extending join, written as a field of
  a record of sequences: `b: birds`.
- **row**: one element of a head or extending join's result, a record with one
  field per side.
- **head form**: a join whose input is a record of sides, which starts a query.
- **extending form**: a join whose input is a sequence of rows and whose
  argument is a record of further sides, which continues a query.
- **key function**: `left` or `right`, mapping one element to the value it is
  matched on.
- **optional side**: a side written `name?: sequence`, which may be missing
  from a row.

### 2. Conditions

Every join except `cross_join` states when elements match, with key functions,
a predicate, or both.

- `left = f, right = g` match `x` with `y` when `f(x) == g(y)`. It is an
  equality join, which a planner MAY execute as a hash or merge join or push
  into a source.
- `on = p` matches when `p` holds. In the binary form `p` takes the pair,
  `(x, y) => …`; in the head and extending forms it takes the row, `r => …`.
- Given both, elements match when the keys are equal **and** `p` holds, as
  `ON a.k = b.k AND …` does in SQL.
- `left` and `right` MUST be given together or not at all, and their result
  types MUST be equal and support `==`.
- A join with neither keys nor `on` MUST be refused, with a message naming
  `cross_join`. An accidental cross product is the most expensive mistake a
  join can make, so asking for one is explicit.
- An **equality key** in `on` is a conjunct of the form `f(r.x) == g(r.y)`,
  where `x` and `y` are different sides and `f` and `g` read only their own
  side. A planner MAY treat it exactly as it treats `left` and `right`, under
  [HQL-0004](../HQL-0004/README.md)'s rule that the result is the one a nested
  evaluation of the condition gives. This is how the head form, which has more
  than two sides, states keys.
- A checker SHOULD warn when every input is a data source and the join has no
  key, stated or recognized.

### 3. The binary form

The pipeline value joins one other sequence, and `into` builds the result.

```hql
birds
| filter(b => b.metadata.genre == "field")
| join(sightings,
       left  = b => b.name,
       right = s => s.species,
       into  = (b, s) => {species: s.species, title: b.title})
```

```text
join<T, U, K, R>(xs: Seq<T>, other: Seq<U>,
                 left: T -> K, right: U -> K,
                 on: (T, U) -> Bool,
                 into: (T, U) -> R)                          : Seq<R>
left_join<T, U, K, R>(…, into: (T, Option<U>) -> R)          : Seq<R>
right_join<T, U, K, R>(…, into: (Option<T>, U) -> R)         : Seq<R>
full_join<T, U, K, R>(…, into: (Option<T>, Option<U>) -> R)  : Seq<R>
```

The condition is the keys, `on`, or both, per section 2.

- `into` MUST be given. It is what keeps this form free of a pair or record
  type: the program builds the value it wants from each matching pair, a
  `Data` literal, a single field, or a further computation.
- The input and `other` are the left and right sides, in that order.
- `left_join` keeps every element of the input; an element with no match
  produces one result, with `none` for the right. `right_join` keeps every
  element of `other` the same way, and `full_join` keeps both.
- Over a stream input, `xs` MAY be a `Stream<T>` when `other` is finite
  (section 9).

### 4. The head form

A query that starts from many sources writes them as a record of sides.

```hql
birds     = source("field_notes")
sightings = source("sightings_table")

{b: birds, s: sightings}
| join(on = r => r.b.name == r.s.species)
| map(r => {species: r.s.species, title: r.b.title})
```

```text
join(sides: {n1: Seq<T1>, …, nk: Seq<Tk>},
     on:    {n1: T1, …, nk: Tk} -> Bool,
     into:  {n1: T1, …, nk: Tk} -> R)                       : Seq<R>
join(sides: {n1: Seq<T1>, …, nk: Seq<Tk>},
     on:    {n1: T1, …, nk: Tk} -> Bool)                    : Seq<{n1: T1, …, nk: Tk}>
```

- A record of sides MUST have at least two fields, and every field MUST be a
  `Seq`. A `Set` side is refused, with a message saying to `sort` it, because a
  set's members have no positions and results are ordered by positions
  (section 7).
- A side MAY be any expression of a sequence type, including a pipeline:
  `{b: birds | filter(…), s: sightings}`.
- The field names are the program's and follow
  [naming](../../wiki/hql/naming.hmd): `snake_case`, distinct within the record.
- `on` receives a whole row and names each side as a field of it; `r.b.name` is
  field access. Keys are stated inside it, as equality conjuncts (section 2).
- `into` MAY be given and takes the row; without it the result is the rows
  themselves. `join(on = p, into = f)` MUST equal `join(on = p) | map(f)`.

### 5. The extending form

A query already under way adds sides to its rows.

```hql
{b: birds, s: sightings}
| join(on = r => r.b.name == r.s.species)
| filter(r => r.s.count > 3)
| join({c: countries}, on = r => r.s.country == r.c.code)
```

```text
join(rows: Seq<R>, sides: {m1: Seq<U1>, …},
     on: R + {m1: U1, …} -> Bool)                           : Seq<R + {m1: U1, …}>
```

- `R` MUST be a row type, and each new side's name MUST differ from every field
  of `R`. The result row is `R` with the new fields after its own. `into` MAY be
  given, as in the head form.
- The extending form is the head form with the input's rows as its first sides:
  `rows | join({c: cs}, on)` MUST equal the head form whose sides are the
  input's sides followed by the new ones. A query may be written in either form, and a planner sees one
  operation.
- What separates the forms is the argument's type. A sequence argument is the
  binary form; a record of sequences is the extending form; no positional
  argument, over a record input, is the head form.

### 6. Optional sides

In the head and extending forms, a side marked `?` may be missing from a row.

```hql
{b: birds, s?: sightings}  | join(on = …)   // every bird; s : Option<Data>
{b?: birds, s: sightings}  | join(on = …)   // every sighting; b : Option<Card>
{b?: birds, s?: sightings} | join(on = …)   // every bird and every sighting
{b: birds, s: sightings}   | join(on = …)   // matches only
```

- In a row type, an optional side `s?: Seq<T>` has type `Option<T>`. A program
  reaches it with the operations of
  [option type](../../wiki/hql/types/option-type.hmd): `unwrap_or`, `is_some`,
  `match`.
- An element of an optional side that matches no row of the other sides MUST
  appear in exactly one row, with every other side it did not match `none`. An
  element of a required side that matches nothing appears in no row.
- `on` reads an optional side as an `Option<T>`, so a comparison over one must
  say what absence means. There is no three-valued logic.
- In the extending form an optional new side, `rows | join({c?: cs}, on)`,
  keeps every input row.

The `?` is the mark [fields](../../wiki/hql/fields.hmd) gives an optional field,
and means the same thing: this may not be here. The binary form spells the same
modes as `left_join`, `right_join` and `full_join`, because it has no record to
mark.

### 7. Order

Results are in an order fixed by the positions of their sides.

- Results MUST be ordered lexicographically by the position of each side's
  element in its sequence, sides compared in order: left before right in the
  binary form, the order the record writes them in the head form.
  `{b: birds, s: sightings}` lists every match of the first bird, in sighting
  order, before any match of the second.
- A missing side sorts after every present position of that side, so unmatched
  elements of a later side follow the matched results.
- Every physical join a planner chooses MUST produce this order, or the planner
  MUST prove the order unobservable — as it is before `size`, `group` or a
  `Set`. A hash join that loses the order restores it.

### 8. Cross, semi and anti joins

```text
cross_join<T, U, R>(xs: Seq<T>, other: Seq<U>, into: (T, U) -> R) : Seq<R>
cross_join(sides: {n1: Seq<T1>, …})                         : Seq<{n1: T1, …}>
semi_join<T, U>(xs: Seq<T>, other: Seq<U>, …condition)      : Seq<T>
anti_join<T, U>(xs: Seq<T>, other: Seq<U>, …condition)      : Seq<T>
```

```hql
{a: sizes, b: colours} | cross_join
birds | semi_join(sightings, left = b => b.name, right = s => s.species)  // seen
birds | anti_join(sightings, left = b => b.name, right = s => s.species)  // never seen
```

- `cross_join` takes no condition. Its order is section 7's.
- `semi_join` and `anti_join` take a condition by section 2 and no `into`. Each
  MUST keep an element of `xs` at most once, in its position, whether it
  matches one element of `other` or many.

### 9. Joining across kinds of source

What a join can do depends on what its sides are, and the sides' types say so.
A finite side has whole, stable contents; a stream side arrives over time and
may never end; a live side, such as a queue, cannot be replayed. The checker
decides every case below from types alone.

| Sides | Example | Modes | Result |
| --- | --- | --- | --- |
| finite × finite | vault × table, table × file | every mode | `Seq`, pure |
| stream × finite | topic × table | `join`, `left_join` keeping the stream, `semi_join`, `anti_join` | `Stream`, in message order |
| live × finite | queue × table | as stream × finite | `Stream`, effectful |
| bounded stream × anything finite | `{a: topic \| take(1000), t: table}` | every mode | `Seq` |
| windowed stream × finite | `topic \| collect(500) \| map(b => b \| join(table, …)) \| emit` | as stream × finite | `Stream` |
| stream × stream | topic × topic | refused | — |

Three rules produce the table.

- **A held side MUST be finite and stable.** A join reads one side as messages
  arrive and holds the others to match against, and only a `FiniteSource` or a
  sequence can be held: a stream has no whole to hold, and a live side cannot be
  read twice. When a held side is a `KeyedSource` and the condition is its key,
  the host MAY look each match up with `get` rather than hold the side.
- **A mode that outputs the unmatched elements of a held side MUST be refused
  when any side is unbounded.** Which elements of a table no message matched is
  known only when the stream ends, and an unbounded stream does not. So
  `right_join` keeping the table, and `full_join`, are refused over a stream.
- **If any side is a stream, the result is a `Stream`, in that side's order; if
  any side is live, the join is `Effectful`.** Section 7's order applies with
  the stream side first.

A stream-to-stream join — messages that arrive within 5 seconds of each other —
needs time windows over both sides. It waits for the durations that
[HQL-0007](../HQL-0007/README.md)'s `collect(…, for = 5s)` waits for.

#### A stream made finite joins as a batch

A stream bound to a name is still a stream. A binding records how to start
traversing its value, not a position already reached, so every use of a live
source waits for new messages: two queries over one queue binding see different
messages. `collect` alone does not change that; `queue | collect(500)` is a
stream of batches.

What makes a stream act as a batch is taking a finite part of it and
materializing that part:

```hql
batch = source("sightings_queue") | take(500) | list   // List<Message<Data>>

{m: batch, t: species}
| join(on = r => r.m.value.species == r.t.name)

batch | anti_join(species, left = m => m.value.species, right = t => t.name)
```

`batch` is a `List`, fixed once taken. Every later query in the program sees
the same 500 messages, every join over it is finite × finite and may use any
mode, and those joins are pure. Only taking the batch was effectful.

When time windows exist, "up to 500, or what arrives in 5 seconds" is one window
of the stream:

```hql
batch = source("sightings_queue") | collect(500, for = 5s) | take(1) | emit
```

A batch lasts for one program. Keeping one across runs is the saved output of a
cell, which [HQL-0009](../HQL-0009/README.md) owns.

#### A stream joined batch by batch

To join a stream continuously without holding it, a program batches it and
joins each batch:

```hql
source("sightings")
| collect(500)
| map(b => b | join(species, left = m => m.value.species, right = t => t.name,
                    into = (m, t) => {species: t.name, at: m.time}))
| emit
```

The result stays a stream, and each batch is matched against the held side in
one request, as SQL's `WHERE name IN (…)` would be.

#### A held source is a snapshot

By [HQL-0012](../HQL-0012/README.md), every use of a finite source in one
evaluation observes the same contents. A long-running join of a stream to a
table therefore matches every message against the table as it was when the
evaluation began. That is what makes the join's answer well-defined. Matching
against a table that changes while the stream runs is a different join, one
whose held side is refreshed on a time bound, and it belongs with time windows.

### 10. Self-joins and branches

A side may be any sequence, so the same source can appear twice under two
names, and a [`tee`](../HQL-0010/README.md) branch can be joined back:

```hql
cards
| filter(c => c.metadata.genre == "field")
| tee recent = filter(c => c.metadata.year >= 2025)
| ...

{all: cards, new: recent} | join(on = r => r.all.name == r.new.name)
```

## Backwards Compatibility

Additive. `join`, `left_join`, `right_join`, `full_join`, `cross_join`,
`semi_join` and `anti_join` are new core names. The record literal gains the
`?` mark on a field, which is new grammar inside an existing form; every record
literal accepted today is accepted unchanged. The nested-`map` workarounds in
issue 0016 keep working and keep their nested results.

## Security Considerations

A join's cost is the product of its sides in the worst case. The run-context
bounds of [HQL-0012](../HQL-0012/README.md) apply to what a join holds, and
exceeding one is an error naming the join, never a truncated result. The
warning of section 2 exists so that an unkeyed join over two large sources is
noticed before it runs.

## Deployment / Activation

1. The binary form, with `left`, `right`, `on` and `into`, inner and the three
   outer modes, evaluated as nested loops. It needs no new type.
2. `cross_join`, `semi_join` and `anti_join` in their binary forms.
3. Row types: a structural record type for a record literal whose fields are
   typed, with field access and the `?` mark.
4. The head and extending forms, with optional sides, checked to agree with
   each other.
5. Equality-key recognition in `on`, and a hash join that keeps the order of
   section 7.
6. Pushdown into a source, with HQL-0012's first database kind.

## Reference Implementation

- `src/extensions/collections.rs` — the join steps, as core steps
- `std/collections.hql` — their signatures beside the existing core operations
- `src/parser.rs` — the `?` mark in a record literal
- `src/types/` — the row type and its field access
- `examples/` — a corpus example per mode and form, and an `invalid` case for a
  join with no condition

## Test Plan

Unit tests MUST include:

- each mode over two small lists with duplicates on both sides, against
  hand-written expected results in the order of section 7
- keys alone, `on` alone, and both, giving the results their definitions do
- an unmatched element of an optional side appearing exactly once, with the
  other side `none`
- `semi_join` keeping an element once when it matches twice
- the extending form equal to the head form over the same sides, and
  `join(on, into)` equal to `join(on) | map(into)`

Integration tests MUST include:

- a vault joined to a fixture table by card name, in the binary and the head
  form, through `tests/corpus.rs`
- a join with no condition, refused with the message naming `cross_join`
- `left` without `right`, refused
- a `Set` side, refused with the message naming `sort`
- two `Stream` sides, refused, and a `full_join` of a stream to a table,
  refused with a message saying the unmatched rows need the stream's end
- a fixture queue taken as `take(5) | list` and joined twice, both joins seeing
  the same five messages
- a stream joined to a table batch by batch through `collect` and `emit`, equal
  to the per-message join

```bash
cargo test
cargo test --test corpus
```

## Open Questions

None.

## Changelog

- 2026-10-18: drafted
