<!-- hql reference: std/graph.Graph -->
# `Graph`

In [`std/graph`](../index.md).

```hql
type Graph<N, E> { nodes : Set<Card> edges : Set<Edge> }
```

A graph: the set of its `nodes`, which are cards, and the set of its
`edges`, the links among them. Both are sets: discovering a graph does
not order it. `vault.graph` is every card of the vault and the links
among them.

```hql
fn Graph(xs: Graph) : Graph
```

`xs` itself: a graph projects to itself.

```hql
fn Graph(xs: Collection<Doc>) : Graph
fn Graph(xs: Collection<Hit<Doc>>) : Graph
```

The graph of the cards in `xs`, or the cards its hits hold, as nodes,
with the vault's edges among them. A relation card is not a node: its
source and target are. `Graph(vault.cards)` has the nodes and edges of
`vault.graph`.

```hql
fn Graph(xs: Collection<Edge>) : Graph
```

The graph of the edges `xs`: their sources and targets as nodes, and the
edges as given.

```hql
fn Graph(x: Doc) : Graph
fn Graph(x: Hit<Doc>) : Graph
```

The graph of the one card `x`, or of the card the hit `x` holds: that
card as its only node, with any vault edge from it to itself.

```hql
fn Graph(nodes: Collection<Doc>, edges: Collection<Edge>) : Graph
```

The graph of `nodes`, together with the source and target of every edge
in `edges`, and the edges as given.
<!-- hql reference: end -->
