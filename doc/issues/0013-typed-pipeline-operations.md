# 0013 — Typed pipeline operations: streams, batching, aggregation, presentation

Status: backlog
Decisions: [HQL-0005](../proposals/HQL-0005/README.md),
[HQL-0006](../proposals/HQL-0006/README.md),
[HQL-0007](../proposals/HQL-0007/README.md),
[HQL-0008](../proposals/HQL-0008/README.md); the reduce side of
[HQL-0004](../proposals/HQL-0004/README.md)
Prompt: [0013-typed-pipeline-operations.prompt.md](0013-typed-pipeline-operations.prompt.md)
Depends on: nothing to start; shares the traversal carrier with
[0010](archive/0010-yield-and-bind.md)

HQL pipelines become general typed programs rather than one MapReduce cycle. A
pipeline may be arbitrarily long and may stream, batch, transform, aggregate and
present in one program. The four records above hold the design; this card holds
the order of the work and what "done" means.

An HQL pipeline is an ordered sequence of typed steps. A step applies an
operation or presentation to the value flowing through the pipeline. The
logical pipeline does not prescribe a particular execution strategy.

## Relation to issue 0012

[0012](0012-mapreduce-backends.md) carries out HQL-0004. Its second commit — the
reduce side — moves here, because `group`, `reduce` and `collect` now have to be
specified against `Stream<T>` and `collect` has changed meaning. 0012 keeps
shape, the backend seam, selection and conformance. Update 0012's sequence when
this card moves to `todo`.

## Sequence

Seven commits, in this order. The first changes no behaviour.

1. **Vocabulary** (HQL-0005). Rename the registry entry `Step` to `Operation`
   and `StepSpec` to `OperationSpec`. Rewrite "stage" to "step" wherever it names
   an occurrence, across `doc/`, `examples/` comments and `src/` doc comments.
   The suite passes with no test edited. Done (feature 10 of
   [0014](archive/0014-next-feature-bundle.md)): two tests that name the registry
   type were renamed with it. The report schema's `stage` field names a
   checking phase, not a pipeline occurrence, and keeps its name.
2. **`Stream<T>`** (HQL-0006). The abstract type in `lib/std/collections/mod.hql`,
   `Seq<T> : {Collection<T>, Stream<T>}`, the stream overloads of `map`,
   `filter` and `take`, and the finite-input diagnostic on `size`, `count`,
   `contains` and `sort`. Existing result types unchanged.
3. **Traversal carrier.** The push traversal described in
   [queries](../models/behavior/queries.md), so that a stream is not forced by
   the steps that consume it, plus an unbounded test-only source for the unit
   tests. Steps 2 and 3 land together with 0010's `yield`, which runs over
   the same carrier.
4. **`collect(n)`, `emit`, `list`** (HQL-0007), with the `Batch` shape and the
   window-shaped argument.
5. **`group`, `reduce`** (HQL-0004's signatures), finite inputs only, with the
   finite-input diagnostic on a stream. `Group<K, T>` as a type. The three
   `status: proposed` cases in `examples/mapreduce/` are promoted. Done for
   finite inputs, with `reduce`'s fold from the first element and `sum`, `min`
   and `max` (feature 2 of [0014](archive/0014-next-feature-bundle.md)); the
   finite-input diagnostic lands with `Stream<T>`.
6. **Presentation** (HQL-0008). The separate presentation table, the
   last-step check, `markdown`, incremental rendering of a stream, and the
   `graph` split.
7. **Corpus and documentation.** The examples below, and every document named
   under Done when.

## Examples

A new corpus folder, `examples/pipelines/`, with one query per file. Each is
executable: it declares `expected-type` and an expected result over a finite
source that is typed as a `Stream<T>`. The prompt's `_.x` shorthand is written
as a lambda, since the grammar has no placeholder, and `processBatch` becomes an
inline lambda, since function values are [0009](archive/0009-function-type-and-constructor.md).

```hql
events
| map(e => e.payload)
```

```hql
events
| filter(e => e.active)
```

```hql
events
| collect(100)
```

```hql
events
| collect(100)
| emit
```

```hql
events
| filter(e => e.active)
| collect(100)
| map(b => sort(b, by = e => e.score))
| emit
| filter(e => e.accepted)
| table
```

The last one exists to show that `collect` is not terminal: a batch boundary
sits in the middle, work is done per batch, and the pipeline returns to single
values before it is presented. Add, beside them:

- a refusal case: `events | sort(by = e => e.at)` fails to check, with the
  message naming `take` and `collect`
- a bounded unbounded case, if the harness can bind a test source:
  `naturals | filter(n => n % 2 == 0) | take(3)` is `[0, 2, 4]`

## Done when

- Every query in `examples/pipelines/` and `examples/mapreduce/` is `valid-now`
  and runs from the command line.
- The Open Questions of HQL-0005 through HQL-0008 are resolved and the four
  records are `accepted`. HQL-0004's `collect` reads `list`, and its open
  question on the name is closed.
- The decisions are stated where they are owned, and nothing describes an
  implemented operation as future work:
  - `lib/std/collections/mod.hql` — `Stream<T>` and every new signature
  - `doc/models/behavior/expressions.md` — the grammar, if `emit`/`table`
    without parentheses or the window argument needs a production
  - `doc/models/behavior/queries.md` — status line updated for what is now
    implemented
  - `doc/models/behavior/map-reduce.md` — vocabulary, `list`, and a pipeline
    that is not one cycle
  - `doc/wiki/hql/types/pipes.hmd` — the six terms and the sentence above
  - `doc/wiki/hql/types/collections/` — a `Stream<T>` card, separate from the
    collection cards
  - `doc/wiki/hql/presentation.hmd` — the rules from HQL-0008
  - `doc/wiki/hql/design-status.hmd` — current status of each operation
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.

## Left open on purpose

Whether `group` on a `Stream<T>` is refused, lifted per batch, or window-bound.
This card implements `group` over finite inputs only and refuses a stream. The
per-batch form is already expressible as
`collect(n) | map(b => group(b, by = ...))`. Deciding the stream semantics is
its own proposal, to be written once HQL's window semantics are settled.
