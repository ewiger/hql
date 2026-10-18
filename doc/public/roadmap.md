# Roadmap

HQL is in language design, and the order of work is chosen so that the type
system settles before the features that lean on it.

## Built

- **Aggregation** — `group`, `reduce`, `sum`, `min` and `max` over finite
  inputs.
- **Functions as values** — function types, `T -> U`, named and recursive
  functions that are bound, passed and returned, and partial application with
  a hole, `add(10, _)`.
- **Streams** — `Stream<T>` as a type, with `map`, `filter`, `take`, `drop`,
  and batching through `collect(n)`, `emit` and `flat_map`.
- **Generated sources** — `yield` in the block body of a `fn`, and `cards` as
  the lazy sequence `vault.cards`.
- **Presentation** — presentation formats as types, `format` and its
  shorthands, configurable renderers, `--to`, and a stream written piece by
  piece as it arrives.
- **A semantic-search doctor** — `hql doctor` checks a vault, its index and its
  model, and names the first missing part.

## Next

- **Cells** — named cells in a document, sharing values through a dependency
  graph, once `hql render` compiles a document.
- **Generic functions** — `fn first<T>(xs: Seq<T>) : Option<T>`.
- **Data sources** — JSON, YAML and CSV read as query sources, with
  `--from <format>`.
- **Record values** — records and the `Table` shape type.
- **Generic graphs** — `Graph<N, E>` beyond the vault's cards.

The designs are recorded as numbered proposals, and the
[design status](../wiki/hql/design-status.hmd) card tracks what is built.
Releases and their changes are in the
[changelog](https://github.com/ewiger/hql/blob/main/CHANGELOG.md).
