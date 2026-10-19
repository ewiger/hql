# Roadmap

HQL is in language design, and the order of work is chosen so that the type
system settles before the features that lean on it.

## Built

- **Aggregation** — `group`, `reduce`, `sum`, `product`, `average`, `min` and
  `max` over finite inputs; combiners `merge`, `union` and `concat`, and
  `reduce(0, +)` with a combiner named directly.
- **Functions as values** — function types, `T -> U`, named, recursive and
  generic functions (`fn first<T>`), aliases whose types the checker writes
  (`top(n) := sort | take(n)`), and partial application with a hole,
  `add(10, _)`.
- **Streams** — `Stream<T>` as a type, with `map`, `filter`, `take`, `drop`,
  and batching through `collect(n)`, `emit` and `flat_map`.
- **Generated sources** — `yield` in the block body of a `fn`, and `cards` as
  the lazy sequence `vault.cards`.
- **Presentation** — presentation formats as types, `format` and its
  shorthands, configurable renderers, `--to`, and a stream written piece by
  piece as it arrives.
- **Cells** — the HQL blocks of a document share values by id and run in
  dependency order.
- **Graphs** — `Graph` from cards, from edges or from both, and the vault's
  graph as `graph`.
- **A naming lint and a semantic-search doctor** — a warning for a name off
  its kind's casing, and `hql doctor`, which names the first missing part of
  the retrieval chain.

## Next

- **Record values** — records, the `Table` shape type, and the joins that read
  them.
- **Data sources** — JSON, YAML and CSV read as query sources, with
  `--from <format>`.
- **Execution backends** — operation shapes, a planner and backends beyond
  the local one.
- **Generic graphs** — `Graph<N, E>` beyond the vault's cards.

The designs are recorded as numbered proposals, and the
[design status](../wiki/hql/design-status.hmd) card tracks what is built.
Releases and their changes are in the
[changelog](https://github.com/ewiger/hql/blob/main/CHANGELOG.md).
