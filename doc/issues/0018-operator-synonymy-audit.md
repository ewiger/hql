# 0018 — Operator synonymy: one name, one signature

Status: done (feature 1 of 0014)
Analysis: [operation signatures compared](../models/behavior/operation-signatures.md)
Decisions: [HQL-0004](../proposals/HQL-0004/README.md),
[HQL-0006](../proposals/HQL-0006/README.md),
[HQL-0007](../proposals/HQL-0007/README.md)
Related: step 2 and step 5 of [0013](0013-typed-pipeline-operations.md),
[0016](0016-sql-parity-analysis.md), [0017](0017-graph-projection-and-view.md),
[HQL-0015](../proposals/HQL-0015/README.md), which states the shorthand rule,
[HQL-0016](../proposals/HQL-0016/README.md), which separates narrowing from
conversion

HQL's operations are declared on several fronts at once: the implemented core,
three proposals, the queries model and the wiki. The comparison linked above
lines up every signature and sorts the pairs by what the types say. Two
operations are the same operation when they share input, arguments, result type
and result value. Names that merely sound alike are not evidence, and the
pairs raised on that basis (`somes`/`emit`, `take`/`collect(n)`,
`uplinks`/`expand`) are closed by their signatures.

The rule for admitting a name: **a shorthand for a specific case of a general
operation is fine when its longhand can be written; a second name for the same
signature and result is not.** `sum` is `reduce(0, +)`; `of_type(U)` is
`filter(x => x | is(U))`; `given … otherwise` is a two-arm `match`. Unary
`count(xs)` for `size(xs)` is the one declared alias and stays.

## Findings

**Same signature, same result** — duplicates, each with a decision:

| Signatures | Decision |
| --- | --- |
| `collect<T>(Seq<T>) : List<T>`, `list<T>(Seq<T>) : List<T>` and the `List(xs)` constructor | `List` (HQL-0016); HQL-0004 and `examples/mapreduce/README.md` still say `collect` |
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
and `List : Seq`, so no program breaks. But `List` after `take` or `sort` is
then an identity in the implementation and a materialization in the model, and
that has to be settled one way.

**Not findings:** `lexical`/`semantic`, `uplinks`/`downlinks` and the three
presenters share a signature and return different values; `map`, `filter`,
`drop`, `collect(n)` and `emit` over several input types are overloads under
one name, as the model intends.

## Work

1. **One signature per name: the result keeps the input's type.** Neither
   `Seq` nor `List` is the answer. Each operation returns the type it was
   given, and two kinds of type parameter write that promise down.

   An operation that keeps the element type is generic over the input type,
   bounded by what the operation needs:

   ```hql
   filter<S : Iterable<T>, T>(xs: S, by: T -> Bool)      : S
   drop<S : Stream<T>, T>(xs: S, n: Int)                 : S
   take<S : Seq<T>, T>(xs: S, n: Int)                    : S
   take<T>(xs: Stream<T>, n: Int)                        : List<T>
   sort<S : Seq<T>, T : Orderable>(xs: S)                : S
   sort<T : Orderable>(xs: Collection<T>)                : List<T>
   ```

   A `List` stays a `List`, a `Seq` a `Seq`, a `Ranking` a `Ranking` (the
   `Ranking` feature of [0014](0014-next-feature-bundle.md)). `take` over a
   stream that is not a `Seq` gains finiteness and returns `List<T>`; `sort`
   over a `Set` gains an order and returns `List<T>`. `xs | List` after `take`
   or `sort` is an identity on a `List` and materializes a `Seq`.

   An operation that changes the element type is generic over the input's
   type constructor:

   ```hql
   map<F<_> : Iterable<_>, T, U>(xs: F<T>, f: T -> U)              : F<U>
   filter<F<_> : Iterable<_>, T, U>(xs: F<T>, by: x => x | is(U))  : F<U>
   ```

   The second is the narrowing `filter` of
   [HQL-0016](../proposals/HQL-0016/README.md), which `of_type(U)` names.

   `F<_> : Iterable<_>` binds a constructor whose argument is its element
   type: `F<X> : Iterable<X>` for every `X`. `List`, `Seq`, `Set` and `Stream`
   qualify, so `map` over a `Set` returns the image `Set<U>`, which can be
   smaller than the input. `Ranking` does not qualify, because
   `Ranking<T> : Seq<Hit<T>>` has `Hit<T>` as its element type; a `Ranking` is
   mapped as the `Seq<Hit<T>>` it is and returns `Seq<U>`.

   Work: write constructor parameters into the
   [type system](../wiki/hql/type-system.hmd) card beside `<T>` and the
   [bind versus apply](../models/behavior/bind-vs-apply-in-generic-types.md)
   model; write these signatures into
   [queries](../models/behavior/queries.md), `std/collections/mod.hql`, the
   registry and HQL-0006; teach the checker to bind `S` and `F<_>`; and give
   `sort` an overload for `Ranking<T>` that returns `Seq<Hit<T>>`, since a
   new order is not the retrieval's
   ([HQL-0016](../proposals/HQL-0016/README.md)).
2. **State the shorthand rule and the tree** on the
   [pipes](../wiki/hql/types/pipes.hmd) card: `reduce` over `sum`, `min`,
   `max`, `size`, `count(x)`; `map` then `emit` over `flat_map`; `filter` over
   `of_type` by way of `is`, and `match` over `is`;
   `match` over `given … otherwise`; `count(xs)` as the declared alias. This
   states on the card the rule and the list
   [HQL-0015](../proposals/HQL-0015/README.md) owns.
3. **Give `somes` a home** on the [option-type](../wiki/hql/types/option-type.hmd)
   card, with its signature `List<Option<T>> : List<T>`, beside the single-value
   option operations.
4. **Finish the `collect` rename.** `examples/mapreduce/README.md` reads
   `List`. HQL-0004's own text is covered by 0013's Done when.
5. **0016 names `emit`, not `flatten`,** in the `CROSS JOIN` and `INNER JOIN`
   rows. `flat_map(f)` is the shorthand for `map(f) | emit`, as
   [HQL-0007](../proposals/HQL-0007/README.md) states, and `flatten` is an alias
   of nothing.
6. **Carry `drop`.** The model declares it and nothing else does; 0016 needs it
   for `OFFSET`. Add it to the stream overloads of HQL-0006 or to 0016's plan.

## Done when

- Every operation name has one signature across `hql builtins`,
  `std/collections/mod.hql`, the queries model and the accepted proposals, and
  the comparison document's "same name, different signatures" table is empty.
- The pipes card states the shorthand rule and the tree, and every operation
  in `hql builtins` appears in it exactly once.
- `somes` is declared on the option-type card.
- No corpus README says `collect` materializes a sequence; 0016 names `emit`.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
