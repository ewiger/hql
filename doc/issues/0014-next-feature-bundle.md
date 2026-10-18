# 0014 — Next feature bundle: signatures and aggregation first, then the pipeline foundations

Status: todo
Branch: `feature/bundle-0014`
Bundles: [0008](0008-cards-as-a-vault-binding.md),
[0009](0009-function-type-and-constructor.md),
[0010](0010-yield-and-bind.md),
[0011](0011-wire-the-semantic-search-toolchain.md),
[0013](0013-typed-pipeline-operations.md),
[0017](0017-graph-projection-and-view.md),
[0018](0018-operator-synonymy-audit.md),
[HQL-0009](../proposals/HQL-0009/README.md),
[HQL-0010](../proposals/HQL-0010/README.md)

Seventeen features, ranked by value per unit of work and ordered so that each
one's prerequisites come before it. Each one is an existing issue, a step of
one, or a designed card that has no issue yet; this card fixes the order and
says what the bundle as a whole delivers. The design lives in the linked
issues, proposals and cards; nothing here restates it.

## Features, in order

1. **One signature per name** — steps 1 to 6 of
   [0018](0018-operator-synonymy-audit.md), against the
   [operation signatures](../models/behavior/operation-signatures.md)
   comparison. `take`, `sort` and `map` read the same in `hql builtins`,
   `std/collections.hql`, the queries model and HQL-0006; "the input's kind"
   gets a written result type; the shorthand rule and tree go on the pipes
   card; `somes` gets its declaration on the option-type card. Documentation
   and std only. Every later feature that adds or changes a signature writes
   it in the form this one settles.
2. **`group` / `reduce` over finite inputs** — the reduce side of
   [HQL-0004](../proposals/HQL-0004/README.md), step 5 of
   [0013](0013-typed-pipeline-operations.md). Needs neither `Stream<T>` nor
   function values: lambdas already work in argument position. `sum`, `min`,
   `max`, `size` and `count(x)` are declared as its shorthands, as the
   signature feature states them. Promotes the three `status: proposed` cases
   in `examples/mapreduce/` (`word-count`, `total-pages`, `by-genre`) to
   `valid-now`.
3. **Scalar operators** — roadblock 1 of [0016](0016-sql-parity-analysis.md):
   comparison beyond equality, boolean operators, the rest of arithmetic, and
   `Ordering` constants. Done, except the `Ordering` constants: `<`, `<=`,
   `>`, `>=`; `and`, `or`, `not` as reserved words; `-`, `*`, `/`, `%`, with
   truncating `Int` division. Grammar work in the
   [expressions](../models/behavior/expressions.md) model. Unblocks `avg`, a
   `HAVING` threshold and every range predicate.
4. **`match` and `given … otherwise`** — the
   [match-op](../wiki/hql/operators/match-op.hmd) card; roadblock 4 of 0016.
   Value arms over `Option`, `Result` and literals, and the two-arm short form.
   `somes` is implemented here, as the step that makes `typed` writable as
   `map(match …) | somes`.
5. **Interpreting `Data`** — the type arm of `match` and the structural test
   over `Data`, from the
   [type-interpretation](../wiki/hql/type-interpretation.hmd) card; roadblock 2
   of 0016, roadblock 3 of [0015](0015-jq-parity-analysis.md). Needs `match`.
   After it, a metadata field can be summed, sorted and compared once it has
   been read as a declared type.
6. **Function values and the `T -> U` type** —
   [0009](0009-function-type-and-constructor.md). Unblocks `yield` / `bind`,
   named functions passed to operations, and the batch-processing examples of
   0013. Reconcile its function type with [0010](0010-yield-and-bind.md)
   before starting.
7. **`cards` as a vault binding** —
   [0008](0008-cards-as-a-vault-binding.md). Small and self-contained; ends the
   silently ignored `cards = [1, 2]` binding. Done.
8. **The `debug` operator** — the
   [debug-op](../wiki/hql/operators/debug-op.hmd) card, with the `debug` and
   `info` severities and the `level` setting from the
   [error-reporting](../wiki/hql/error-reporting.hmd) card. An identity step
   whose capture goes to the report log, ordered by trace. Done; `fail` and
   the language's own `code` / `trace` fields stay designed.
9. **Pipeline vocabulary rename** — step 1 of 0013,
   [HQL-0005](../proposals/HQL-0005/README.md): `Step` to `Operation`, "stage"
   to "step". No behaviour change.
10. **`tee`** — [HQL-0010](../proposals/HQL-0010/README.md): `input | tee x =
    branch | next` binds the branch's result and passes the input on
    unchanged. The `localhost` backend evaluates the shared input once. Needs
    the vocabulary rename, so the new step node is named in the new
    vocabulary.
11. **`Stream<T>` as a type** — step 2 of 0013,
    [HQL-0006](../proposals/HQL-0006/README.md): the declaration, stream
    overloads of `map`, `filter`, `take` and `drop`, and the finite-input
    diagnostic. `drop` is carried here, as step 6 of 0018 asks.
12. **Semantic-search `doctor`** —
    [0011](0011-wire-the-semantic-search-toolchain.md): one check that walks
    vault, config, index, model and query set, and names the first missing
    part.
13. **`Ranking` as an ordinary declared subtype** — the pending decision in
    `doc/memory/`. `Ranking<T> : Seq<Hit<T>>` stays: it states where a
    sequence's order came from, which a bare `Seq<Hit<T>>` does not. What
    goes is its special-casing in the implementation:
    - Done: the type system has applied parents, and declares
      `Ranking<T> : Seq<Hit<T>>` with one. Subtyping, the element type,
      `is_sequence` and `with_element` go through it; no `"Ranking"` string
      check is left in `src/types/builtin.rs` or `src/extensions/spec.rs`.
    - Done: `take` keeps any concrete sequence's type, which is what keeps a
      ranking a ranking; the `Ranking` exception is gone. Writing that rule as
      a signature waits for the signature feature.
    - Left: the carrier is generic over its element, so `Value::type_of`
      reports the element a ranking was built from and `Ranking<Concept>`,
      which `recall` needs, can exist.
    - Left, and not a `Ranking` change: built-in record fields, those of
      `Ranking`, `Hit`, `Card` and `Graph` alike, are read from declarations
      rather than from the table in `checker::field_type`.
14. **`yield` / `bind`** — [0010](0010-yield-and-bind.md): generated sequences
    as query sources. Needs function values and the traversal carrier (step 3
    of 0013).
15. **`collect(n)`, `emit`, `list`** — step 4 of 0013,
    [HQL-0007](../proposals/HQL-0007/README.md). Needs `Stream<T>`.
16. **Presentation separate from computation** — step 6 of 0013,
    [HQL-0008](../proposals/HQL-0008/README.md). The document host part, the
    `markdown` presentation and the pipe-table form of `table`, is carried by
    [0020](0020-render-as-compilation.md). What remains is the last-step check,
    incremental rendering of a stream, and the `graph` split of
    [0017](0017-graph-projection-and-view.md): `graph` becomes the
    presentation and the projection gets its own name.
17. **Cells** — [HQL-0009](../proposals/HQL-0009/README.md): cell `id`s,
    values shared through the cell dependency graph, saved outputs with a
    source hash, `--cell` and the per-cell JSON record. Needs 0020 done.

## Not in this bundle

These stay in their own issues and are not ticked here:

- [0020](0020-render-as-compilation.md), `hql render` as compilation, which is
  in `doing` on its own; the presentation and cells features build on it.
- The rest of [0016](0016-sql-parity-analysis.md)'s roadblocks: record and pair
  values, `flat_map`, `join` and set operations, the window steps other than
  `drop`, other sources and writes.
- Format input and output from [0015](0015-jq-parity-analysis.md).
- Execution backends from [0012](0012-mapreduce-backends.md).

## Working rule

Pick features top to bottom. When one starts, move its own issue to `doing`
(or, for a 0013 or 0018 step, note the step there); this card stays in `todo`
and tracks the order. A feature with no issue of its own (scalar operators,
`match`, interpreting `Data`, `debug`) gets one when it starts. Tick a feature
here when its issue or step is done.

- [ ] 1. One signature per name
- [ ] 2. `group` / `reduce` over finite inputs
- [x] 3. Scalar operators
- [ ] 4. `match` and `given … otherwise`
- [ ] 5. Interpreting `Data`
- [ ] 6. Function values
- [x] 7. `cards` as a vault binding
- [x] 8. The `debug` operator
- [ ] 9. Pipeline vocabulary rename
- [ ] 10. `tee`
- [ ] 11. `Stream<T>`
- [ ] 12. Semantic-search `doctor`
- [ ] 13. `Ranking` as an ordinary declared subtype (subtyping done; carrier left)
- [ ] 14. `yield` / `bind`
- [ ] 15. `collect(n)`, `emit`, `list`
- [ ] 16. Presentation
- [ ] 17. Cells

## Done when

- Every feature above is ticked, and each bundled issue is `done` or has had
  its remaining scope moved to a new card.
- Every case in `examples/mapreduce/` is `valid-now`.
- No `"Ranking"` string comparison is left in `src/types/` or
  `src/extensions/spec.rs`, and a ranking can hold any element type.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
