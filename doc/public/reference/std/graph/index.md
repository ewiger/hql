<!-- hql reference: std/graph -->
# `std/graph`

```hql
module graph
```

Graphs: nodes and the edges among them, structural only; what a node or
an edge means is the `knowledge` module's. There is no `Node` type: what
a node is arrives as a type parameter, so a graph over documents and a
graph over concepts share one declaration.

- [`Edge`](types/Edge.md) — An edge from `source` to `target`, with `data` about it.
- [`Graph`](types/Graph.md) — A graph: the set of its `nodes`, which are cards, and the set of its `edges`, the links among them.
- [`Link`](types/Link.md) — An authored link from one document to another: an `Edge` between cards.
- [`HmdGraph`](types/HmdGraph.md) — The document graph: a `Graph` of cards and the authored `Link`s among them.
- [`uplinks`](uplinks.md) — The edges of the links `x` writes to other cards: one `Edge` per authored link, its `source` the card `x`.
- [`downlinks`](downlinks.md) — The edges of the links other cards of the vault write to `x`: one `Edge` per authored link, its `target` the card `x`.
- [`expand`](expand.md) — The graph reached from `x` by following links `depth` steps, `1` unless given, in `direction`: `"uplinks"`, `"downlinks"` or `"both"`, the default.
<!-- hql reference: end -->
