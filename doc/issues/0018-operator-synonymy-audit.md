# 0018 — Operator synonymy: one name, one signature

Status: backlog
Analysis: [operation signatures compared](../models/behavior/operation-signatures.md)
Decisions: [HQL-0004](../proposals/HQL-0004/README.md),
[HQL-0006](../proposals/HQL-0006/README.md),
[HQL-0007](../proposals/HQL-0007/README.md)
Related: step 2 and step 5 of [0013](0013-typed-pipeline-operations.md),
[0016](0016-sql-parity-analysis.md), [0017](0017-graph-projection-and-view.md),
the shorthand note in `doc/memory/`

HQL's operations are declared on several fronts at once: the implemented core,
three proposals, the queries model and the wiki. The comparison linked above
lines up every signature and sorts the pairs by what the types say. Two
operations are the same operation when they share input, arguments, result type
and result value. Names that merely sound alike are not evidence, and the
pairs raised on that basis (`somes`/`emit`, `take`/`collect(n)`,
`uplinks`/`expand`, `filter`/`typed`) are closed by their signatures.

The rule for admitting a name: **a shorthand for a specific case of a general
operation is fine when its longhand can be written; a second name for the same
signature and result is not.** `sum` is `reduce(0, +)`; `typed(U)` is
`map(match …) | somes`; `given … otherwise` is a two-arm `match`. Unary
`count(xs)` for `size(xs)` is the one declared alias and stays.

## Findings

**Same signature, same result** — duplicates, each with a decision:

| Signatures | Decision |
| --- | --- |
| `collect<T>(Seq<T>) : List<T>` and `list<T>(Seq<T>) : List<T>` | `list` (HQL-0007); HQL-0004 and `examples/mapreduce/README.md` still say `collect` |
| `graph(x) : Graph` and `expand(x, depth = 0) : Graph` | 0017: `graph` becomes the presentation, the projection is renamed |
| `emit<T>(Seq<Seq<T>>) : List<T>` and the `flatten` 0016 asks for | `emit` (HQL-0007); 0016's join rows name the wrong operation |

**Same name, different signatures** — one operation declared inconsistently:

| Name | Declarations |
| --- | --- |
| `take` over a stream | `List<T>` in HQL-0006, `Seq<T>` in the queries model |
| `sort` | `List<T>` in std and the registry, `Seq<T>` in the model |
| `map` over a finite input | `Collection<T> → List<R>` in the registry, `Seq<T> → Seq<U>` in the model |
| `filter`, `typed` | return "the input's kind", for which the design has no written type |
| `take` on a concrete `Seq` | the rule is a help string, not a signature |
| `count` | the alias form is in the registry and not in the model |

The model's rule is that a step returns the least promise that is still true,
and `List : Seq`, so no program breaks. But `list` after `take` or `sort` is
then an identity in the implementation and a materialization in the model, and
that has to be settled one way.

**Not findings:** `lexical`/`semantic`, `uplinks`/`downlinks` and the three
presenters share a signature and return different values; `map`, `filter`,
`drop`, `collect(n)` and `emit` over several input types are overloads under
one name, as the model intends.

## Work

1. **One signature per name.** Decide whether the queries model keeps `Seq`
   as the least promise or adopts the registry's `List`, and write the result
   into [queries](../models/behavior/queries.md), `std/collections.hql` and
   HQL-0006 so that `take`, `sort` and `map` read the same everywhere. State
   what `map` returns from a `Set`, give the "input's kind" result a written
   type or spell each case out, and write the rule for `take` into a signature:
   a concrete `Seq` keeps its type, which is what keeps a `Ranking` a ranking
   (the `Ranking` feature of [0014](0014-next-feature-bundle.md)).
2. **State the shorthand rule and the tree** on the
   [pipes](../wiki/hql/types/pipes.hmd) card: `reduce` over `sum`, `min`,
   `max`, `size`, `count(x)`; `match` over `typed` by way of `somes`;
   `match` over `given … otherwise`; `count(xs)` as the declared alias. This
   digests the shorthand note in `doc/memory/`, which is then deleted.
3. **Give `somes` a home** on the [option-type](../wiki/hql/types/option-type.hmd)
   card, with its signature `List<Option<T>> : List<T>`, beside the single-value
   option operations.
4. **Finish the `collect` rename.** `examples/mapreduce/README.md` reads
   `list`. HQL-0004's own text is covered by 0013's Done when.
5. **0016 names `emit`, not `flatten`,** in the `CROSS JOIN` and `INNER JOIN`
   rows. Whether `flat_map(f)` earns a name as the shorthand for `map(f) | emit`
   is decided in 0016's own step on joins, under the rule here.
6. **Carry `drop`.** The model declares it and nothing else does; 0016 needs it
   for `OFFSET`. Add it to the stream overloads of HQL-0006 or to 0016's plan.

## Done when

- Every operation name has one signature across `hql builtins`,
  `std/collections.hql`, the queries model and the accepted proposals, and
  the comparison document's "same name, different signatures" table is empty.
- The pipes card states the shorthand rule and the tree, and every operation
  in `hql builtins` appears in it exactly once.
- `somes` is declared on the option-type card and the shorthand note is gone
  from `doc/memory/`.
- No corpus README says `collect` materializes a sequence; 0016 names `emit`.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
