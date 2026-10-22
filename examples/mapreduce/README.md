# MapReduce: a shelf to group and reduce

Start at [the library index](wiki/library.hmd). Six book cards with a `genre`
and a page count are the dataset for filtering, mapping, grouping and reducing.
Only `wiki/` is the vault, so this README and the query files do not become
cards. The vault's `hql.toml` names the `localhost` backend, which is the
default and the only one; the line records where the results are promised to
hold rather than changing anything, and `hql config` reports it.

From the repository root:

```sh
cargo build --locked
target/debug/hql --vault examples/mapreduce/wiki run examples/mapreduce/queries/novels.hql
target/debug/hql --vault examples/mapreduce/wiki run examples/mapreduce/queries/shelf.hql
target/debug/hql --vault examples/mapreduce/wiki run examples/mapreduce/queries/by-genre.hql
target/debug/hql run examples/mapreduce/queries/word-count.hql
target/debug/hql --vault examples/mapreduce/wiki run examples/mapreduce/queries/genre-counts.hql
target/debug/hql run examples/mapreduce/queries/page-histogram.hql
target/debug/hql --vault examples/mapreduce/wiki render examples/mapreduce/wiki/library.hmd
```

Add `--envelope json` before `run` to inspect the inferred type and structured
result.

| Query | What it shows | Status | Result |
| --- | --- | --- | --- |
| [novels](queries/novels.hql) | Three steps — filter, sort, map — run in order | runs | Dune, Frankenstein, The Hobbit |
| [shelf](queries/shelf.hql) | `size` as a reduction; one filter per genre, by hand | runs | 6 books: 3 novels, 2 essays, 1 poetry |
| [word count](queries/word-count.hql) | `group` by identity, then `map` over the groups | runs | owl 3, raven 1, robin 1 |
| [total pages](queries/total-pages.hql) | `reduce` as a left fold over a sequence | runs | 1589 |
| [by genre](queries/by-genre.hql) | The full package as one pipeline over card metadata | runs | poetry 1, essay 2, novel 3 |
| [metadata pages](queries/metadata-pages.hql) | A metadata leaf read as an `Int` with `of_type`, then summed | runs | 1589 |
| [tee summary](queries/tee-summary.hql) | `tee` binds side counts while the main line counts the novels | runs | 3 novels, 6 books, 2 short |
| [merge layers](queries/merge-layers.hql) | `merge` as a combiner: layers of metadata folded in order, a later layer winning | runs | shelf fiction, 14 loan days |
| [genre counts](queries/genre-counts.hql) | `count_by` as one reduction, the answer of by genre without `group` | runs | essay 2, novel 3, poetry 1 |
| [page histogram](queries/page-histogram.hql) | `histogram(width)`: counts per bucket, in key order | runs | 0: 1, 100: 1, 200: 1, 300: 2, 400: 1 |
| [longest books](queries/longest-books.hql) | `top_k(k, by)`: the k greatest by a key, greatest first | runs | Dune, The Common Reader, The Hobbit |
| [page variance](queries/page-variance.hql) | `variance`: a state of count, mean and squared deviations, then a quotient | runs | 10738.8… |
| [distinct genres](queries/distinct-genres.hql) | `approx_distinct`: a deterministic HyperLogLog estimate | runs | 3 |
| [reserved operation](queries/invalid-reserved-operation.hql) | The operation types, one per shape, are reserved names | refused | a type error |

Every `|` is a boundary the executor sees, and every step has a shape, which
`hql builtins` prints beside its purity: `filter` and `map` are elementwise,
`group` is a partition, `size` and `sum` are reductions, `sort` is a barrier. A
`reduce` is a reduction when its `by` names a combiner, `reduce(0, +)`, and a
barrier when it is a lambda, as in [total pages](queries/total-pages.hql).
Under `localhost` each step runs to completion before the next,
with no parallelism, and that sequential answer is the oracle every other
backend has to match.

- `group(by = …)` partitions a collection by key into a `Set<Group<K, T>>`; a
  group has a `key` and its `items`, a `List<T>` in input order. Two groups are
  equal iff their keys are.
- `reduce(initial, by)` folds a `Seq<T>` from the left; `reduce(by)` folds from
  the first element and returns `Option<T>`, absent over an empty input. Both
  refuse a `Set`, which has no positions to fold along; sort first.
- `sum`, `min` and `max` are the shorthands `reduce(0, +)`, `reduce(min)` and
  `reduce(max)`. They accept any collection, since their combiners do not
  depend on order.
- `average`, `variance`, `count_by`, `histogram`, `top_k` and `approx_distinct`
  carry a state of another type than their element and project it at the
  end; each state combines by a combiner the host holds, so each is a
  reduction. `std/collections.hql` writes each one's longhand fold beside its
  signature.
- The `List` constructor materializes a `Seq<T>` into a `List<T>`, as
  `List(xs)`. No query here needs it, because nothing in the implemented slice
  is lazy.

[Shelf](queries/shelf.hql) and [by genre](queries/by-genre.hql) compute the same
answer, one with a filter per genre and one with `group`. The comparison is the
point: `group` is not a convenience but the step at which a backend may
repartition, and the sorted result is deliberately ordered by an explicit
comparison of group sizes because a genre read from metadata is a `Data` leaf,
which is not `Orderable`.

The step vocabulary, the backend contract, and the three steps are specified
in [HQL-0004](../../doc/proposals/HQL-0004/README.md).
