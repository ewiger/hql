# 0014 — Next feature bundle: aggregation first, then the pipeline foundations

Status: todo
Bundles: [0008](0008-cards-as-a-vault-binding.md),
[0009](0009-function-type-and-constructor.md),
[0010](0010-yield-and-bind.md),
[0011](0011-wire-the-semantic-search-toolchain.md),
[0013](0013-typed-pipeline-operations.md)

Ten features, ranked by value per unit of work. Each one is either an existing
issue or a step of one; this card fixes the order and says what the bundle as a
whole delivers. The design lives in the linked issues and proposals; nothing
here restates it.

## Features, in order

1. **`group` / `reduce` over finite inputs** — the reduce side of
   [HQL-0004](../proposals/HQL-0004/README.md), step 5 of
   [0013](0013-typed-pipeline-operations.md). Needs neither `Stream<T>` nor
   function values: lambdas already work in argument position. Promotes the
   three `status: proposed` cases in `examples/mapreduce/`
   (`word-count`, `total-pages`, `by-genre`) to `valid-now`.
2. **Function values and the `T -> U` type** —
   [0009](0009-function-type-and-constructor.md). Unblocks 0010, named
   functions passed to operations, and the batch-processing examples of 0013.
   Reconcile its function type with [0010](0010-yield-and-bind.md) before
   starting.
3. **`cards` as a vault binding** —
   [0008](0008-cards-as-a-vault-binding.md). Small and self-contained; ends the
   silently ignored `cards = [1, 2]` binding.
4. **Pipeline vocabulary rename** — step 1 of
   [0013](0013-typed-pipeline-operations.md),
   [HQL-0005](../proposals/HQL-0005/README.md): `Step` to `Operation`, "stage"
   to "step". No behaviour change.
5. **`Stream<T>` as a type** — step 2 of 0013,
   [HQL-0006](../proposals/HQL-0006/README.md): the declaration, stream
   overloads of `map`, `filter`, `take`, and the finite-input diagnostic.
6. **Semantic-search `doctor`** —
   [0011](0011-wire-the-semantic-search-toolchain.md): one check that walks
   vault, config, index, model and query set, and names the first missing part.
7. **`yield` / `bind`** — [0010](0010-yield-and-bind.md): generated sequences
   as query sources. Needs 2 and the traversal carrier (step 3 of 0013).
8. **`collect(n)`, `emit`, `list`** — step 4 of 0013,
   [HQL-0007](../proposals/HQL-0007/README.md). Needs 5.
9. **Presentation separate from computation** — step 6 of 0013,
   [HQL-0008](../proposals/HQL-0008/README.md): the last-step check,
   `markdown`, incremental rendering of a stream.
10. **Retire the `Ranking` constructor** — the pending decision in
    `doc/memory/`: narrow `Hit<T>` and weigh the loss of `ranking.hits`,
    `ranking.query` and `ranking.retrieval` field types in the checker.

## Working rule

Pick features top to bottom. When one starts, move its own issue to `doing`
(or, for a 0013 step, note the step there); this card stays in `todo` and
tracks the order. Tick a feature here when its issue or step is done.

- [ ] 1. `group` / `reduce` over finite inputs
- [ ] 2. Function values
- [ ] 3. `cards` as a vault binding
- [ ] 4. Pipeline vocabulary rename
- [ ] 5. `Stream<T>`
- [ ] 6. Semantic-search `doctor`
- [ ] 7. `yield` / `bind`
- [ ] 8. `collect(n)`, `emit`, `list`
- [ ] 9. Presentation
- [ ] 10. Retire `Ranking`

## Done when

- Every feature above is ticked, and each bundled issue is `done` or has had
  its remaining scope moved to a new card.
- Every case in `examples/mapreduce/` is `valid-now`.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
