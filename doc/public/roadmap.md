# Roadmap

The order of work is chosen so that the type system settles before the
features that lean on it.

## Built

- **Aggregation** — `group`, `reduce`, `sum`, `product`, `average`, `min`,
  `max`, `variance`, `count_by`, `histogram`, `top_k` and `approx_distinct`
  over finite inputs; combiners `merge`, `union` and `concat`, and
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

## In the next release

- **I/O** — endpoints (`stdin`, `stdout`, `stderr`, `file`), and `source`,
  `tap`, `target` and `sink` over JSON, YAML, CSV and text; channels of typed
  values; a capability policy for documents. See
  [Reading and writing data](learn/13-reading-and-writing.md).
- **Logging** — loggers over any endpoint, levels as types, and `--log-level`.
  See [Channels and logs](learn/14-channels-and-logs.md).
- **AI** — agents as cards, `Context`, `Prompt` and `Tool`, runs kept as
  values, delegation, memory as cards, and recordings that replay offline. See
  [Agents and runs](learn/15-agents-and-runs.md).

## Next

- **Record values** — the `Table` shape type, and the joins that read
  records.
- **The network** — `http`, `websocket` and `socket` endpoints, and configured
  sources and targets by id.
- **Execution backends** — operation shapes, a planner and backends beyond
  the local one.
- **Generic graphs** — `Graph<N, E>` beyond the vault's cards.

The designs are recorded as numbered proposals, and the
[design status](../wiki/hql/design-status.hmd) card tracks what is built.
Releases and their changes are in the
[changelog](https://github.com/ewiger/hql/blob/main/CHANGELOG.md).
