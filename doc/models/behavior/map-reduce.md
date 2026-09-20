# Map/reduce for queries

Status: **design, partly implemented.** `filter`, `map`, `size` and `count` run
today, sequentially. `reduce`, `group`, `collect`, the shape of a step and the
backend that runs a program are specified by
[HQL-0004](../../proposals/HQL-0004/README.md) and carried out by
[issue 0012](../../issues/0012-mapreduce-backends.md). The example that pins
the intended results is [`examples/mapreduce/`](../../../examples/mapreduce/README.md).

HQL does not implement MapReduce in the Hadoop sense. The claim is narrower and
more useful: **the pipe is a map/reduce-like execution boundary.** A chain of
pipes is a region in which the executor may treat the program as a graph of
transformations and reductions rather than as nested function calls, as long as
the typed semantics are preserved.

## The pipe is a stage boundary

`x | f(a)` is typed exactly as `f(x, a)`, and every boundary checks one thing:

```text
output type of the left stage  :  input type of the right stage
```

The generics belong to the functions being piped and are inferred; `|` carries
none. What the pipe adds is not a type but a node: it lowers to a step in the
execution tree and is never rewritten into a call, because the node is what
tells a backend that here is a boundary it may plan around.

```hql
cards
| filter(c => c.metadata.genre == "novel")
| map(c => c.title)
| size
```

reads as `Set<Card> → Set<Card> → List<String> → Int`, and also as three stages
with two boundaries.

## Stages have a shape

Purity says whether a step may move. Shape says whether it may be split, and it
is the whole of what an executor may assume:

```text
Elementwise   filter, map, typed        run over any split; recombine in order
Partition     group                     route by key; groups finish in any order
Reduction     size, count, contains     reduce each split; combine with a known combiner
Barrier       sort, take, reduce,       the whole input first; nothing may be split
              collect, get, retrieval,
              presentation
```

`reduce` is a barrier and not a reduction because its combining function is the
program's own, and an arbitrary function cannot be assumed associative. The
reductions are the steps whose combiner the executor holds.

## The reduce side

```hql
type Group<K, T> { key : K, items : List<T> }

reduce<T, R>(xs: Seq<T>, initial: R, by: (R, T) -> R) : R
group<T, K>(xs: Collection<T>, by: T -> K) : Set<Group<K, T>>
collect<T>(xs: Seq<T>) : List<T>
```

`reduce` is a left fold and takes a `Seq` because a fold visits positions in
order; a `Set` has none and is sorted first. `group` returns a `Set` because
partitions finish in no promised order, and the type says so rather than a
backend sorting to keep a promise the program never needed. `collect` is the
point at which a lazy sequence becomes a value; on today's values it is the
identity.

Word count is the example the vocabulary is named after:

```hql
words
| group(by = w => w)
| sort(by = g => g.key)
| map(g => { word: g.key, count: size(g.items) })
```

## Backends

A checked program is handed to a backend. `localhost` is the standard one: the
in-memory evaluator, each stage run to completion before the next, with no
parallelism. It is always registered, it is the default, and it is the oracle —
every other backend must produce its value, at its type, with its positions and
its diagnostic. Selection is configuration (`[execution] backend` in `hql.toml`,
`--backend`, `HQL_BACKEND`), never syntax, because the answer does not depend on
it.

Within a region a backend may fuse adjacent elementwise stages, split a run of
elementwise stages and one reduction across partitions, repartition at a group
and run what follows per group, and must assemble everything at a barrier.
`localhost` takes none of these permissions. That is what lets a vault of fifty
cards and a vault of a million run the same program.

See [queries](queries.md) for the collection hierarchy that says which
reductions are legal — `size` needs finiteness, `take` needs order — and
[pipes](../../wiki/hql/types/pipes.hmd) for the execution freedom this model
gives a shape to.
