# HQL-0004: MapReduce: pipeline stages and execution backends

**Status**: drafted
**Created**: 2026-09-20
**Source**: [map/reduce for queries](../../models/behavior/map-reduce.md)

## Abstract

The pipe `|` is defined as a **stage boundary**: `x | f(a)` is typed exactly as
`f(x, a)` and lowers to a step node that the executor still sees, never to a
call. Beside its purity, every step declares a **shape** — `Elementwise`,
`Partition`, `Reduction` or `Barrier` — which is the whole of what an executor
may assume about splitting it. The reduce side of the vocabulary joins the core:
`reduce` folds a sequence, `group` partitions a collection into a
`Set<Group<K, T>>`, and `collect` materializes a sequence. A checked program is
handed to an **execution backend**. The standard backend is `localhost`: the
in-memory evaluator, one stage after another, one traversal per stage, with no
parallelism at all. It is always registered, it is the default, and it is the
oracle every other backend is measured against. Which backend runs a program is
host configuration, never syntax. On the Rust side the three steps and the
backend contract are organized as one module, `mapreduce`, and that
organization is a host detail that does not reach HQL's surface.

## Motivation

The knowledge base has been promising this for some time without anything
consuming the promise:

- [pipes](../../wiki/hql/types/pipes.hmd) reserves "lazy evaluation, step
  fusion, source-side query execution and partitioned work" as executor
  options, and [HQL-0001](../HQL-0001/README.md) gave every step a `Purity`
  because that freedom is only safe over pure steps.
- [architecture](../../models/domain/architecture.md) states: "Execution is
  still sequential: a planner, parallel scheduling, and backend pushdown are
  future consumers of the typed execution boundary." There is a boundary and
  no consumer.
- [map/reduce for queries](../../models/behavior/map-reduce.md) says the pipe
  is a map/reduce-like execution boundary and that a chain of pipes "forms a
  query execution region". It is the only place the idea lives, and it is a
  note rather than a decision.

Three concrete things are wrong today, and they are why this is a record rather
than a commit.

**The vocabulary is half there.** The core has `filter` and `map`, and it has
`size` and `count`, which are reductions. It has no `reduce`, no `group` and no
materialization step. A pipeline can select and transform, and it can count,
and it cannot aggregate. The queries that would exercise the reserved execution
freedom cannot be written, so the freedom has never been tested against a
value. `examples/mapreduce/shelf.hql` computes a count per genre with one
filter per genre, which is the shape `group` exists to replace.

**Purity is not enough to split a step.** A pure step may be reordered, but
nothing says whether it may be *partitioned*. `map` may be run on halves of its
input and the halves concatenated; `sort` may not; `size` may, but only because
its combiner is a sum the executor knows. That distinction is what a backend
needs and what no declaration currently carries. Without it, an executor is
either forbidden everything or has to guess from the step's name.

**There is no place for a backend to exist.** [HQL-0002](../HQL-0002/README.md)
declines approximate search pushed into a store because pipes forbids an
optimization that changes observable values, and defers the question to its
own proposal. A PostgreSQL extension that pushes `filter` and `take` into SQL,
or a threaded executor that partitions a vault of a million cards, would each
need the same three things: to receive the checked tree, to know which stages
they may claim, and to be held to the same answer `localhost` gives. This
proposal defines those three things and builds only `localhost`, so that a
vault of fifty cards and a vault of a million run the same program and the
difference is a line in `hql.toml`.

## Goals

- A pipeline reads as a chain of stages whose boundaries the executor sees.
- Aggregation is expressible: word count is a pipeline.
- Every step says how it may be split, and the executor never guesses.
- A second backend can be added without touching the checker, the evaluator or
  a program, and is held to `localhost`'s answer by a test.

## Non-goals

- **No parallelism is built.** `localhost` is sequential by definition, and it
  is the only backend this proposal delivers. A threaded or distributed backend
  is a later proposal that plugs into the seam defined here.
- **No planner.** The legal rewrites over a region are enumerated so that a
  backend knows its permissions; nothing here applies them.
- **No pushdown.** A backend that hands stages to an external store — SQL, an
  ANN index — is out of scope, and the rule that would let it claim a prefix of
  a region is an open question, not a specification.
- **No `Pipe<A, B>` type, and no generics on `|`.** The generics belong to the
  functions being piped, and are inferred. Nothing new appears in the type
  lattice except `Group<K, T>`.
- **No laziness.** `collect` is specified because a materialization boundary
  must have a name, and it is the identity on today's values; observing it
  needs generated sequences, which [issue 0010](../../issues/0010-yield-and-bind.md)
  owns.
- **No function values.** `reduce` takes a lambda argument as `sort` and
  `filter` do. Passing a named function is [issue 0009](../../issues/0009-function-type-and-constructor.md).

## Specification

### 1. The pipe is a stage boundary

The pipe has two readings, and both are normative.

- For **typing**, `x | f(a, b)` MUST be checked exactly as `f(x, a, b)`: the
  output type of the left side must satisfy the input type the step declares,
  and the step's generic parameters are bound from it. There is no type
  annotation on `|` and none is ever required.
- For **execution**, a pipe MUST lower to a step node in the execution tree and
  MUST NOT be rewritten into a call before a backend sees it. The node is what
  carries the semantic information that this is a boundary, and a rewrite that
  loses it loses the ability to fuse, partition or push stages.
- A **region** is a maximal chain of steps joined by `|`. The rewrites in
  section 5 are permitted within a region and nowhere else.
- A direct call, `size(xs)`, is an ordinary call and not a stage. It produces
  the same value as `xs | size`; only the execution freedom differs. The pipe
  is the way a program says "this is a query", and a call is the way it says
  "this is a function".

```text
Language semantics:   x | f(y)  ≡  f(x, y)
Query semantics:      | opens a stage boundary the backend sees
```

Conceptually the operator is polymorphic, `(|)<A, B> : A -> (A -> B) -> B`;
its parameters are never written.

### 2. Shape

Every step declares a shape beside its purity, because purity says whether a
step may move and shape says whether it may be split.

```text
Shape
    Elementwise    the result for each element depends on that element alone
    Partition      the input is regrouped by a key; groups may complete in any order
    Reduction      the whole input collapses to one value through a known combiner
    Barrier        the whole input is needed before any output; nothing may be split
```

- A step MUST declare exactly one shape. It is a static property of the step,
  not of the input type; `map` is elementwise over a `Set` and over a `Seq`.
- `Elementwise` permits running the step over any split of the input and
  recombining in the input's order. `filter`, `map` and `typed` are elementwise.
- `Partition` permits routing each element to a partition by key and running
  the partitions independently. `group` is the one partition step.
- `Reduction` permits reducing each split and combining the results, and only
  when the combiner is one the *executor* holds: `size` combines by addition,
  `count` by addition, `contains` by disjunction. A step whose combining
  function is supplied by the program is NOT a reduction, because an arbitrary
  function cannot be assumed associative; `reduce` is a barrier.
- `Barrier` permits nothing. `sort`, `take`, `reduce`, `collect`, `get` and
  every retrieval and presentation step are barriers. `take` is a barrier
  because a prefix of a split is not a prefix of the whole.
- Shape is host metadata. It MUST be printed by `hql builtins` beside purity,
  and it MUST NOT be writable in HQL. The [type system](../../wiki/hql/type-system.hmd)
  rule that a trait is a host mechanism applies to shape unchanged.

The core steps, after this proposal:

| Step | Purity | Shape | Input | Output |
| --- | --- | --- | --- | --- |
| `filter` | pure | elementwise | `Collection<T>` | same collection |
| `map` | pure | elementwise | `Collection<T>` | `List<R>` |
| `typed` | pure | elementwise | `Collection<T>` | same collection, narrowed |
| `group` | pure | partition | `Collection<T>` | `Set<Group<K, T>>` |
| `size` | pure | reduction | `Collection<T>` | `Int` |
| `count` | pure | reduction | `Collection<T>` | `Int` |
| `contains` | pure | reduction | `Collection<T>` | `Bool` |
| `reduce` | pure | barrier | `Seq<T>` | `R` |
| `sort` | pure | barrier | `Collection<T>` | `List<T>` |
| `take` | pure | barrier | `Seq<T>` | `List<T>` |
| `collect` | pure | barrier | `Seq<T>` | `List<T>` |
| `get` | pure | barrier | `Map<K, V>` | `Option<V>` |

### 3. The reduce side of the vocabulary

Three steps and one type join the core. They are core rather than an imported
extension because they operate on the core's own collection types and mention
nothing above them, which is the rule [HQL-0001](../HQL-0001/README.md) set.

```hql
type Group<K, T> {
    key   : K
    items : List<T>
}

reduce<T, R>(xs: Seq<T>, initial: R, by: (R, T) -> R) : R
group<T, K>(xs: Collection<T>, by: T -> K) : Set<Group<K, T>>
collect<T>(xs: Seq<T>) : List<T>
```

- `reduce` MUST be a left fold: `by(by(by(initial, x0), x1), x2)` and so on to
  the last position. It MUST accept only a `Seq<T>`, because a fold visits
  positions in order and a `Set` has none; reducing a set is a type error whose
  message says to sort first. The alternative — accepting any collection and
  leaving the order unspecified — would make `reduce("", concat)` over a set
  produce a different string on different backends, which is exactly the
  non-determinism this proposal exists to rule out.
- `group` MUST place each occurrence in the group of its key, using HQL
  equality on keys, and MUST return a `Set` of groups. It returns a set because
  a partitioned execution finishes its partitions in no promised order, and
  the type says so rather than a backend having to sort to keep a promise the
  program never needed. Two groups are equal iff their keys are. Within a
  group, `items` MUST keep the relative positions of the input when the input
  is a `Seq<T>`; over an unordered input the list's order is not a guarantee.
- `collect` MUST return a `List<T>` holding the sequence's occurrences at
  their positions. Over a `List<T>` it is the identity. It is a barrier because
  it is the point at which a lazy sequence becomes a value; on today's values
  it does nothing observable, and it is specified now so that the boundary has
  a name before [issue 0010](../../issues/0010-yield-and-bind.md) makes it
  visible.
- `Group<K, T>` is a record with exactly two fields. `key` has the type the
  selector returned; `items` is a `List<T>` of the input's element type.
  Sorting groups by key needs `K : Orderable`, as any key selector does; a key
  read from card metadata is a `Data` leaf, which is not, so those groups are
  ordered by an explicit comparison — see `examples/mapreduce/by-genre.hql`.

Word count, the example this vocabulary is named after:

```hql
words
| group(by = w => w)
| sort(by = g => g.key)
| map(g => { word: g.key, count: size(g.items) })
```

`group` is the partition, the `map` over groups is where each group is reduced
independently, and `sort` is the barrier that makes the answer positional.

### 4. Backends

A backend is what a checked program is executed by.

```text
Backend
    name()   -> &str
    run(program: &execution::Program, vault: &Vault) -> Result<(Value, Warnings), Diagnostic>
```

- A backend MUST receive the checked execution tree and nothing earlier. It
  MUST NOT parse or re-check; checking has already run over the whole program,
  and a backend that could see source could disagree with the checker.
- A backend MUST produce the value `localhost` produces, at the same type, with
  the same sequence positions. This is the contract that makes a backend a
  backend rather than a different language.
- A backend MUST fail when `localhost` fails, with the same diagnostic. Where
  more than one element would fail, it MUST report the one `localhost` reports,
  which is the earliest in traversal order. A partitioned backend can stop at
  its first failure, but before reporting it has to know that no earlier
  position failed. The rule is strong on purpose: a diagnostic that depends on
  which backend ran is a diagnostic that cannot be pinned by a test.
- A backend MAY apply the rewrites of section 5 within a region, over pure
  steps, according to their shapes, and nowhere else.
- An `Effectful` step MUST run once, where it is written, in program order.
- A backend that cannot run a program MUST refuse it before running any part
  of it, with a diagnostic naming the backend and the step it cannot run.
  Running half a program and then failing over would make effects and
  warnings depend on how far the backend got.
- Backends are compiled in and registered in one table, as extensions are. No
  dynamic loading is introduced.

`localhost` is the standard backend.

- Its name is `localhost`, and it MUST always be registered.
- It is the in-memory evaluator: a tree walk over the execution tree, each
  stage run to completion before the next, one traversal per stage, with no
  fusion, no partitioning and no parallelism. It applies none of the rewrites
  it is permitted; it is defined by its result, and the simplest execution is
  the one least likely to be wrong about it.
- It is the default when nothing selects a backend.
- It is the oracle. A conformance test MUST run every corpus case through every
  registered backend and compare each result to `localhost`'s. With one backend
  the test is a tautology; it is written first so that the second backend
  arrives into a test rather than writing one.

Selection is host configuration, with the precedence
[reporting](../../models/behavior/reporting.md) already uses:

1. the query — `--backend <name>`
2. the vault — `hql.toml`, `[execution] backend = "<name>"`
3. the user — `HQL_BACKEND` in the environment
4. the default — `localhost`

```toml
[execution]
backend = "localhost"
```

- A name that is not registered MUST fail every program with a diagnostic
  naming the source that supplied it — the flag, the file or the variable —
  as a bad `import` in `hql.toml` does.
- `hql config` MUST report the backend in force and which layer supplied it.
- There is no syntax for choosing a backend, because the answer does not
  depend on it and a program that named one would be claiming otherwise.

### 5. What a backend may do within a region

These are the permissions the shapes grant. Nothing here is an obligation, and
`localhost` takes none of them.

- Adjacent pure `Elementwise` stages MAY be fused into one traversal.
- A run of pure `Elementwise` stages, optionally followed by one `Reduction`,
  MAY be split across partitions of the input; elementwise results are
  recombined in the input's order and reductions are combined with the
  executor's combiner.
- A `Partition` stage MAY repartition the input by its key, and the pure
  stages that follow it, up to the next barrier, MAY run per group.
- A `Barrier` ends any split: the whole input is assembled before it runs, and
  what follows starts from its output.
- No rewrite may cross an `Effectful` step, duplicate one or elide one.
- No rewrite may change a value, a sequence position or which diagnostic is
  reported; section 4 already requires this and the rewrites are legal only
  because they preserve it.

```text
cards
| filter(book)        elementwise ─┐
| map(genre)          elementwise ─┼─ one fused traversal, splittable
| group(by = k => k)  partition   ─┘  ↓ repartition by key
| map(summarize)      elementwise     ↓ per group
| sort(by = ...)      barrier         ↓ assemble, then one traversal
```

### 6. Help

- `hql builtins` MUST show shape beside purity for every step, so that what a
  backend may do to a step is visible where the step is documented.
- The signature lines for `reduce`, `group` and `collect` appear in
  `std/collections.hql` when this proposal is accepted, beside `Group<K, T>`.

### 7. The corpus

The example is `examples/mapreduce/`, already in the corpus, with a vault of
six book cards and five queries. Two run today and are `valid-now`; three
need the steps in section 3 and carry a header the harness now admits:

```text
// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0004/README.md
```

- A proposed case MUST declare `expected-type` and one expected result, as a
  `valid-now` case does: it is a pinned intention, not a sketch.
- A proposed case MUST fail to check today, and `tests/corpus.rs` asserts it.
  The day it checks, the assertion names it and its header is promoted in the
  same commit. This is what keeps the corpus from holding a case that claims a
  step does not exist after it does.

## Backwards Compatibility

Additive. Three step names join the core; a step is only ever read after `|`,
so a binding named `group` or `reduce` in an existing program neither collides
nor shadows, by the rule [HQL-0001](../HQL-0001/README.md) set. `Group` joins
the type lattice; a program declaring its own `Group` is refused as any
redeclaration of a binary type is. `[execution]` in `hql.toml` is ignored by a
binary that predates it. `hql::run` keeps its signature and goes through
`localhost`, which is observably today's evaluator; the whole suite passes with
no test edited, and that is the check that the seam changed nothing.

The harness change is additive: `status: proposed` is a third status, and no
existing case carries it.

## Security Considerations

`localhost` adds no surface: no network, no file the vault did not already
name, no code loaded at run time. `group` materializes every input element
into a group, which is bounded by the input a program could already hold in a
`List`. A backend that reaches outside the process — a store, a thread pool —
MUST state its resource bounds and its host contract in the proposal that
introduces it; this record gives it a seam, not a permission.

## Deployment / Activation

1. add `Shape` to `Step`, declare it on every registered step, and print it in
   `hql builtins` — no evaluator change, no test edited
2. add `Group<K, T>`, `reduce`, `group` and `collect` to the core, and promote
   the three proposed corpus cases to `valid-now` in the same commit
3. add the `Backend` trait and `localhost` wrapping today's evaluator, and route
   `hql::run` through it — the whole suite passes unchanged
4. add selection: `[execution]`, `--backend`, `HQL_BACKEND`, and `hql config`
5. add the conformance test over every registered backend
6. only then may a second backend be proposed

Step 3 is the one worth insisting on: the seam is only safe if it is first
observably a no-op.

## Reference Implementation

- `src/extensions/spec.rs`, `src/extensions/mod.rs` — `Shape`, declared on
  `Step` beside `Purity`
- `src/extensions/mapreduce.rs` — `reduce`, `group`, `collect`, registered in
  `CORE`; the one Rust module that holds the map/reduce vocabulary
- `src/types/mod.rs`, `src/types/value.rs`, `src/execution.rs` — the `Group`
  constructor, its value carrier, and `key` and `items` as resolved fields
- `src/backends/mod.rs`, `src/backends/localhost.rs` — the `Backend` trait,
  the registry, and `localhost` over `evaluation::evaluate`
- `src/lib.rs` — `run` selects a backend; `src/cli.rs` — `--backend`,
  `[execution]`, `hql config`
- `src/builtins.rs` — the shape column
- `std/collections.hql` — `Group<K, T>` and the three signatures
- `tests/corpus.rs` — already admits proposed cases; gains the conformance
  test over registered backends

## Test Plan

Unit tests MUST include:

- every registered step declares a shape, and the shape of each core step is
  the one in the table of section 2
- `reduce` over `[412, 280, 310, 172, 320, 95]` with addition is `1589`; over
  a `Set<Int>` it is a type error whose message says to sort first
- `group` over `["owl", "raven", "owl", "robin", "owl"]` by identity yields
  three groups whose `items` sizes are 3, 1 and 1, and whose `key`s are the
  three words; over a `List` the items keep their positions
- `collect` over a `List` is the identity, at the same type
- an unregistered backend name fails with a diagnostic naming the flag, the
  file or the variable that supplied it

Integration tests MUST include:

- every corpus case, including `examples/mapreduce/`, passing through
  `localhost` unchanged
- every registered backend agreeing with `localhost` on every corpus case
- `hql builtins` printing a shape for every step
- `hql config` reporting `localhost` and `default` when nothing selects one,
  and `hql.toml` when the vault does

```bash
cargo build --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Open Questions

These MUST be resolved before this moves from drafted to accepted.

- Is the materialization step `collect` or `list`?
  [Queries](../../models/behavior/queries.md) names it `list`; this record
  names it `collect`, after the map/reduce vocabulary. One name survives.
- Should a scalar `Data` leaf be `Orderable`? Grouping by card metadata is the
  dominant use of `group`, and its keys are `Data`, so today the groups can be
  ordered only by a comparison and never by key. The alternative is a partial
  schema that narrows `metadata.genre` to `String`, which
  [fields](../../wiki/hql/fields.hmd) proposes and nothing implements.
- Is shape a property of the step, or of the step at an input type? `take`
  over a `Seq` is a barrier; `take` over an already-materialized `List` could
  be elementwise on a prefix. The static choice is simpler and is what this
  record makes; it may be too coarse.
- Should a direct call be a one-stage region? `size(xs)` and `xs | size` are
  the same value with different execution freedom, which is defensible and
  also surprising.
- Does `localhost` materialize between stages, or is push traversal
  ([queries](../../models/behavior/queries.md)) a property of `localhost`
  rather than of a lazier backend? `localhost` is defined by its result, so
  either is conformant; the question is which one the name promises.
- Is the failure rule — the earliest failing position, always — too strong for
  a partitioned backend, given that it forces ordered completion before a
  diagnostic can be reported?
- May a backend claim a prefix of a region and hand the remainder to
  `localhost`? That rule is what pushdown needs, and it is the one rule here
  that changes which code runs a stage. It is left out so that this record
  does not decide pushdown by implication.
- Which second backend is first: `threads`, an in-process partitioned executor
  that exercises every rewrite in section 5, or `postgres`, a pushdown that
  exercises the prefix rule? The conformance test is the same for either.

## Changelog

- 2026-09-20: drafted
