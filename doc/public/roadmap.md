# Roadmap

HQL is in language design, and the order of work is chosen so that the type
system settles before the features that lean on it.

## Now: settling the type system

The collection-order vocabulary, function types, and how a pipeline's types
compose are being fixed first. Most of what follows depends on them.

## Next: typed pipeline operations

- **Aggregation** — `group` and `reduce` over finite inputs.
- **Functions as values** — a function type, `T -> U`, and named functions that
  can be bound, passed and returned.
- **Streams** — `Stream<T>` as a type, so a pipeline can run over an unbounded
  source, with batching through `collect(n)` and `emit`.
- **Generated sources** — `yield`, so a query can start from a
  sequence a function produces.
- **Presentation** as a layer separate from computation, with incremental
  rendering of a stream.
- **A semantic-search doctor** — one command that checks a vault, its index
  and its model, and names the first missing part.

The designs are recorded as numbered proposals, and the
[design status](../wiki/hql/design-status.hmd) card tracks what is built.
Releases and their changes are in the
[changelog](https://github.com/ewiger/hql/blob/main/CHANGELOG.md).
