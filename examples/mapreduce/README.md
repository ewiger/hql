# MapReduce: a shelf to group and reduce

Start at [the library index](wiki/library.hmd). Six book cards with a `genre`
and a page count are the dataset for filtering, mapping, grouping and reducing.
Only `wiki/` is the vault, so this README and the query files do not become
cards. The vault's `hql.toml` names the `localhost` backend, which is the
default and the only one; the line records where the results are promised to
hold rather than changing anything.

From the repository root:

```sh
cargo build --locked
target/debug/hql --vault examples/mapreduce/wiki run examples/mapreduce/queries/novels.hql
target/debug/hql --vault examples/mapreduce/wiki run examples/mapreduce/queries/shelf.hql
target/debug/hql --vault examples/mapreduce/wiki render examples/mapreduce/wiki/library.hmd
```

Add `--format json` before `run` to inspect the inferred type and structured
result.

| Query | What it shows | Status | Result |
| --- | --- | --- | --- |
| [novels](queries/novels.hql) | Three stages — filter, sort, map — run in order | runs | Dune, Frankenstein, The Hobbit |
| [shelf](queries/shelf.hql) | `size` as a reduction; one filter per genre, by hand | runs | 6 books: 3 novels, 2 essays, 1 poetry |
| [word count](queries/word-count.hql) | `group` by identity, then `map` over the groups | proposed | owl 3, raven 1, robin 1 |
| [total pages](queries/total-pages.hql) | `reduce` as a left fold over a sequence | proposed | 1589 |
| [by genre](queries/by-genre.hql) | The full package as one pipeline over card metadata | proposed | poetry 1, essay 2, novel 3 |

The two queries that run are ordinary today; what this example adds is the
reading of them as **stages**. Every `|` is a boundary the executor sees:
`filter` and `map` are elementwise, `size` is a reduction, `sort` is a barrier.
Under `localhost` each stage runs to completion before the next, with no
parallelism, and that sequential answer is the oracle every other backend has
to match.

The three proposed queries are written ahead of their implementation. Their
headers pin the type and the result they are meant to produce, and the corpus
harness checks that they *fail* to check today — a proposed case that starts
passing has to be promoted to `valid-now` in the same commit. They need three
steps the core does not yet have:

- `group(by = …)` partitions a collection by key into a `Set<Group<K, T>>`; a
  group has a `key` and its `items`.
- `reduce(initial, by = …)` folds a `Seq<T>` from the left. It refuses a `Set`,
  which has no positions to fold along; sort first.
- `collect` materializes a `Seq<T>` into a `List<T>`. No query here needs it
  yet, because nothing in the implemented slice is lazy.

[Shelf](queries/shelf.hql) and [by genre](queries/by-genre.hql) compute the same
answer, one with a filter per genre and one with `group`. The comparison is the
point: `group` is not a convenience but the stage at which a backend may
repartition, and the sorted result is deliberately ordered by an explicit
comparison of group sizes because a genre read from metadata is a `Data` leaf,
which is not `Orderable`.

The stage vocabulary, the backend contract, and the three steps are specified
in [HQL-0004](../../doc/proposals/HQL-0004/README.md).
