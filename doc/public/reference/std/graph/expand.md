<!-- hql reference: std/graph.expand -->
# `expand`

In [`std/graph`](index.md).

```hql
fn expand(x: Option<Doc>, depth: Int = 1, *, direction: String = "both") : Graph
fn expand(x: Graph, depth: Int = 1, *, direction: String = "both") : Graph
fn expand(x: Doc, depth: Int = 1, *, direction: String = "both") : Graph
fn expand(xs: Collection<Doc>, depth: Int = 1, *, direction: String = "both") : Graph
fn expand(xs: Collection<Hit<Card>>, depth: Int = 1, *, direction: String = "both") : Graph
```

The graph reached from `x` by following links `depth` steps, `1` unless
given, in `direction`: `"uplinks"`, `"downlinks"` or `"both"`, the
default. A depth of `0` is refused: `Graph(xs)` is the graph with no
traversal. It needs a vault.
<!-- hql reference: end -->
