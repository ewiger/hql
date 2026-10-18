# 0014 — Next feature bundle: signatures and aggregation first, then the pipeline foundations

Status: todo (work in progress on `feature/bundle-0014`)
Branch: `feature/bundle-0014`
Bundles: [0008](0008-cards-as-a-vault-binding.md),
[0009](0009-function-type-and-constructor.md),
[0010](0010-yield-and-bind.md),
[0011](0011-wire-the-semantic-search-toolchain.md),
[0013](0013-typed-pipeline-operations.md),
[0017](0017-graph-projection-and-view.md),
[0018](0018-operator-synonymy-audit.md),
[0021](0021-of-type-replaces-typed.md),
[HQL-0009](../proposals/HQL-0009/README.md),
[HQL-0010](../proposals/HQL-0010/README.md),
[HQL-0014](../proposals/HQL-0014/README.md),
[HQL-0017](../proposals/HQL-0017/README.md),
[HQL-0018](../proposals/HQL-0018/README.md)

Eighteen features, ranked by value per unit of work and ordered so that each
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
   card; `somes` gets its declaration on the option-type card. `Iterable<T>`,
   the bound the signatures use, is declared in `std/collections.hql`. The
   checker follows the signatures: `map` over a `Set` returns a `Set`, `sort`
   keeps a concrete `Seq`'s type, and `sort` over a `Ranking<T>` returns
   `Seq<Hit<T>>`. The registry's output forms already express "the input's
   type" and "the input's constructor", so no constructor-kinded parameter is
   added to the type system. The narrowing `filter` is written here and built
   by the `is` / `of_type` feature; `drop`'s signature is written here and
   built with `Stream<T>`. Every later feature that adds or changes a
   signature writes it in the form this one settles.
2. **`group` / `reduce` over finite inputs** — the reduce side of
   [HQL-0004](../proposals/HQL-0004/README.md), step 5 of
   [0013](0013-typed-pipeline-operations.md), with `reduce`'s two forms as
   [HQL-0014](../proposals/HQL-0014/README.md) states them: a fold from an
   initial value, and a fold from the first element returning `Option<T>`.
   Needs neither `Stream<T>` nor function values: lambdas already work in
   argument position. `sum`, `min`, `max`, `size` and `count(x)` are stated
   as its shorthands; until `:=` exists, `sum`, `min` and `max` are registered
   steps whose results equal their longhands, as 0021 does for `of_type`.
   Named combiners (`reduce(0, +)`), `merge` and the combiner-property table
   stay in HQL-0014. Promotes the three `status: proposed` cases in
   `examples/mapreduce/` (`word-count`, `total-pages`, `by-genre`) to
   `valid-now`.
3. **Scalar operators** — roadblock 1 of [0016](0016-sql-parity-analysis.md):
   comparison beyond equality, boolean operators, the rest of arithmetic, and
   `Ordering` constants. Done, except the `Ordering` constants: `<`, `<=`,
   `>`, `>=`; `and`, `or`, `not` as reserved words; `-`, `*`, `/`, `%`, with
   truncating `Int` division. Grammar work in the
   [expressions](../models/behavior/expressions.md) model. Unblocks `avg`, a
   `HAVING` threshold and every range predicate.
4. **`is` and `of_type`** — [0021](0021-of-type-replaces-typed.md), by
   [HQL-0016](../proposals/HQL-0016/README.md): the `is(U)` test, the
   narrowing `filter`, `of_type(U)` as a registered step until `:=` exists,
   and `typed` removed. A dependency: the narrowing `filter` signature of
   feature 1 and the structural test of feature 6 both need it.
5. **`match` and `given … otherwise`** — the
   [match-op](../wiki/hql/operators/match-op.hmd) card; roadblock 4 of 0016.
   Value arms over `Option`, `Result` and literals, and the two-arm short form.
   `somes` is implemented here, beside `match` over `Option`.
6. **Interpreting `Data`** — the type arm of `match` and the structural test
   over `Data`, from the
   [type-interpretation](../wiki/hql/type-interpretation.hmd) card; roadblock 2
   of 0016, roadblock 3 of [0015](0015-jq-parity-analysis.md). Needs `match`,
   and `of_type`, which passes a `Data` tree whose shape a record declares.
   After it, a metadata field can be summed, sorted and compared once it has
   been read as a declared type.
7. **Function values and the `T -> U` type** —
   [0009](0009-function-type-and-constructor.md). Unblocks `yield` / `bind`,
   named functions passed to operations, and the batch-processing examples of
   0013. Reconcile its function type with [0010](0010-yield-and-bind.md)
   before starting, and its `fn` declaration with the `UpperCamelCase`
   constructor functions of HQL-0018 and the `:=` sugar of
   [HQL-0015](../proposals/HQL-0015/README.md), which desugars to it.
8. **`cards` as a vault binding** —
   [0008](0008-cards-as-a-vault-binding.md). Small and self-contained; ends the
   silently ignored `cards = [1, 2]` binding. Done.
9. **The `debug` operator** — the
   [debug-op](../wiki/hql/operators/debug-op.hmd) card, with the `debug` and
   `info` severities and the `level` setting from the
   [error-reporting](../wiki/hql/error-reporting.hmd) card. An identity step
   whose capture goes to the report log, ordered by trace. Done; `fail` and
   the language's own `code` / `trace` fields stay designed.
10. **Pipeline vocabulary rename** — step 1 of 0013,
   [HQL-0005](../proposals/HQL-0005/README.md): `Step` to `Operation`, "stage"
   to "step". No behaviour change.
11. **`tee`** — [HQL-0010](../proposals/HQL-0010/README.md): `input | tee x =
    branch | next` binds the branch's result and passes the input on
    unchanged. The `localhost` backend evaluates the shared input once. Needs
    the vocabulary rename, so the new step node is named in the new
    vocabulary.
12. **`Stream<T>`, `yield` and `bind`** — steps 2 and 3 of 0013,
    [HQL-0006](../proposals/HQL-0006/README.md) and
    [0010](0010-yield-and-bind.md), as one feature: the `Stream<T>`
    declaration, stream overloads of `map`, `filter`, `take` and `drop`, the
    finite-input diagnostic, the push traversal carrier, and generated
    sequences as query sources. An iterating function declares `Seq<T>` or
    `Stream<T>`, so the type and the first source that is genuinely unbounded
    land together. `drop` is carried here, as step 6 of 0018 asks. Needs
    function values.
13. **Semantic-search `doctor`** —
    [0011](0011-wire-the-semantic-search-toolchain.md): one check that walks
    vault, config, index, model and query set, and names the first missing
    part.
14. **`Ranking` as an ordinary declared subtype** — the pending decision in
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
    - Done: the carrier is generic over its element, so `Value::type_of`
      reports the element a ranking was built from and `Ranking<Concept>`,
      which `recall` needs, can exist. A hit's retrieved value is `h.value`,
      as `Hit<T>` declares; `h.card` is refused naming `value`.
    - Moved to [0023](0023-built-in-fields-from-declarations.md), and not a
      `Ranking` change: built-in record fields read from declarations, which
      first needs the declarations and the checker's table reconciled.
15. **Constructors as pipeline steps** — section "Two forms of one call" of
    [HQL-0018](../proposals/HQL-0018/README.md): `x | T(a)` is `T(x, a)`,
    and a bare type name in step position is its constructor. A dependency:
    materialization is `xs | List` once `list` is retired, and the graph
    projection is `cards | Graph`.
16. **`collect(n)`, `emit` and `xs | List`** — step 4 of 0013,
    [HQL-0007](../proposals/HQL-0007/README.md), with materialization written
    as the `List` constructor, as HQL-0016 retires `list`. Needs `Stream<T>`
    and constructors as pipeline steps.
17. **Presentation formats** — [HQL-0017](../proposals/HQL-0017/README.md),
    which supersedes HQL-0008 and step 6 of 0013: the `format` extension in
    place of `present`, the format types, the shorthands, renderers and their
    configuration, the presentation context with `--to`, `--from` and
    `--envelope`, and incremental formatting of a stream. The `graph` split of
    [0017](0017-graph-projection-and-view.md) is the `Graph` constructor
    here; what `Graph` accepts beyond cards is
    [HQL-0019](../proposals/HQL-0019/README.md), not this bundle. Needs
    constructors as pipeline steps, `Stream<T>`, and 0020's document host for
    the `hmd` context.
18. **Cells** — [HQL-0009](../proposals/HQL-0009/README.md): cell `id`s,
    values shared through the cell dependency graph, saved outputs with a
    source hash, `--cell` and the per-cell JSON record. Needs 0020 done.

## Not in this bundle

These stay in their own issues and are not ticked here:

- [0020](0020-render-as-compilation.md), `hql render` as compilation, which is
  in `doing` on its own; the presentation and cells features build on it, and
  cells cannot start until it is done. Its markdown presentation step is
  written against HQL-0008 and moves to HQL-0017's `markdown` shorthand.
- `:=` ([HQL-0015](../proposals/HQL-0015/README.md)). Shorthands it would
  define are registered steps until it lands.
- Graph construction beyond cards ([HQL-0019](../proposals/HQL-0019/README.md)).
- The rest of [HQL-0014](../proposals/HQL-0014/README.md): named combiners,
  `merge`, combiner properties and execution shape.
- The rest of [0016](0016-sql-parity-analysis.md)'s roadblocks: record and pair
  values, `flat_map`, `join` and set operations, the window steps other than
  `drop`, other sources and writes.
- Format input and output from [0015](0015-jq-parity-analysis.md).
- Execution backends from [0012](0012-mapreduce-backends.md).

## Working rule

Pick features top to bottom. When one starts, move its own issue to `doing`
(or, for a 0013 or 0018 step, note the step there); this card stays in `todo`
and tracks the order. A feature with no issue of its own (scalar operators,
`match`, interpreting `Data`, `debug`, constructors as pipeline steps,
presentation formats) gets one when it starts. Tick a feature
here when its issue or step is done.

- [x] 1. One signature per name
- [x] 2. `group` / `reduce` over finite inputs
- [x] 3. Scalar operators (the `Ordering` constants left; nothing waits on them)
- [x] 4. `is` and `of_type`
- [x] 5. `match` and `given … otherwise`
- [x] 6. Interpreting `Data` (leaves and sequences; record schemas wait for 0016's record values)
- [x] 7. Function values (generic declarations moved to 0022)
- [x] 8. `cards` as a vault binding
- [x] 9. The `debug` operator
- [x] 10. Pipeline vocabulary rename
- [x] 11. `tee`
- [ ] 12. `Stream<T>`, `yield` and `bind`
- [x] 13. Semantic-search `doctor`
- [x] 14. `Ranking` as an ordinary declared subtype (field table moved to 0023)
- [x] 15. Constructors as pipeline steps
- [ ] 16. `collect(n)`, `emit` and `xs | List`
- [ ] 17. Presentation formats
- [ ] 18. Cells

## Done when

- Every feature above is ticked, and each bundled issue is `done` or has had
  its remaining scope moved to a new card.
- Every case in `examples/mapreduce/` is `valid-now`.
- No `"Ranking"` string comparison is left in `src/types/` or
  `src/extensions/spec.rs`, and a ranking can hold any element type.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
