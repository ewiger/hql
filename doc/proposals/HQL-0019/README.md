# HQL-0019: Graph construction

- **Status**: drafted
- **Created**: 2026-10-18
- **Source**: [graph](../../wiki/hql/types/graph-type.hmd),
  [HQL-0017](../HQL-0017/README.md), [HQL-0018](../HQL-0018/README.md)

## Abstract

A graph is built by the constructors of `Graph`, by joins whose results become
edges, and by traversal. The general constructor takes nodes and edges,
`Graph(nodes, edges)`, and its node set is the given nodes together with every
edge's endpoints, so `Graph(edges)` is a graph from edges alone. Three
shorthands cover the common sources, each written in terms of the general one:
`Graph(cards)` is the document graph induced by a set of cards and their
authored links; `Graph(rows, source = …, target = …)` turns each row of a
[join](../HQL-0013/README.md) into an edge; `Graph(xs, by = (a, b) => …)`
connects every ordered pair of elements a predicate accepts. A join over two
selections of one vault, `{b: birds, h: habitats}`, gives a bipartite graph.
`expand` grows a graph along the vault's links. A graph exposes its nodes and
edges as sets, so it can be measured and read like any other value.
`KnowledgeGraph` keeps its own constructor in the knowledge extension. The
current vault carries both of its sources as fields, and the standard library
binds them: `cards = vault.cards` and `graph = vault.graph`, the vault's
knowledge graph.

## Motivation

- [HQL-0017](../HQL-0017/README.md) replaces the `graph` step with the `Graph`
  constructor and [HQL-0018](../HQL-0018/README.md) makes a constructor usable
  in a pipeline. What the constructor accepts is still the single case
  `graph` handled: a collection of cards.
- Graphs come from more places than a vault's links. A join of birds to
  habitats, a table of flights, a similarity threshold between cards: each
  states a relation that a program wants to see as a graph, and today none of
  them can become one.
- The [graph](../../wiki/hql/types/graph-type.hmd) card leaves open whether a
  graph's nodes and edges are readable. A graph that cannot be read can only be
  rendered.
- Today's projection keeps every edge of the vault whatever cards it was
  given, so `cards | filter(…) | graph` draws links to cards the filter
  removed.

## Non-goals

- **No graph algorithms.** Paths, components, centrality and subgraph
  matching are operations over a built graph and belong to their own record.
- **No graph mutation.** A graph is immutable, as the graph card requires;
  adding to one builds a new one.
- **No layout.** Drawing is a renderer's, by HQL-0017.

## Specification

### 1. The general constructor

```hql
fn Graph<N, E : Edge<N, N>>(nodes: Collection<N>, edges: Collection<E>) : Graph<N, E>
fn Graph<N, E : Edge<N, N>>(edges: Collection<E>)                        : Graph<N, E>
```

- The node set MUST be `nodes` together with the source and target of every
  edge. An edge never refers to a node the graph lacks, and a node with no
  edges is kept.
- `Graph(edges)` MUST equal `Graph([], edges)`.
- Nodes MUST be identified by HQL equality of `N`; a card is identified by its
  name. Edges are identified as the graph card states for their type: a `Link`
  by its endpoints, so the document graph is simple; a `Relation` by its own
  identity, so the knowledge graph is a multigraph.
- `Edge(source = a, target = b, data = d)` builds an edge, by HQL-0018's record
  constructor.

### 2. The document graph from cards

```hql
fn Graph(cards: Collection<Card>) : HmdGraph
```

- The nodes MUST be the given cards, and the edges the authored links whose
  source **and** target are both among them: the subgraph the cards induce.
  Edges to cards outside the input are not kept.
- A relation card MUST contribute its edge and its two endpoints, and MUST NOT
  be a node, as the [knowledge](../../wiki/hql/knowledge.hmd) card requires.
- Over hits or a ranking, each node MUST record why it is present: a member of
  the input, retrieved with its score, or reached by `expand` from a seed at a
  depth. The renderers of HQL-0017 MAY use it.
- `cards | uplinks | Graph` is a different graph: every outgoing edge of the
  cards, with the targets as nodes wherever they are. The two forms answer
  "how are these connected" and "what do these point at".

### 3. A graph from a join

Each row of a join can become one edge:

```hql
fn Graph<R, S, T>(rows: Seq<R>, source: R -> S, target: R -> T,
                  data: R -> Data = r => {}) : Graph<union {S, T}, Edge<S, T>>
```

```text
rows | Graph(source = f, target = g, data = d)
    ≡  rows | map(r => Edge(source = f(r), target = g(r), data = d(r))) | Graph
```

- It MUST equal its longhand above.
- A join's rows are the usual input. Two selections of one vault, joined, give
  a bipartite graph:

```hql
{b: cards | of_type(Bird), h: cards | of_type(Habitat)}
| join(on = r => r.b | uplinks | map(e => e.target) | contains(r.h))
| Graph(source = r => r.b, target = r => r.h)
```

- Data from two sources joins the same way: `{a: airports, f: flights}` joined
  on the origin code, with `source = r => r.f.origin` and
  `target = r => r.f.destination`, is a route graph.

### 4. A graph from a predicate

```hql
fn Graph<N>(nodes: Collection<N>, by: (N, N) -> Bool) : Graph<N, Edge<N, N>>
```

```text
xs | Graph(by = p)
    ≡  Graph(xs, {a: xs, b: xs} | join(on = r => r.a != r.b and p(r.a, r.b))
                                | map(r => Edge(source = r.a, target = r.b, data = {})))
```

- It MUST equal its longhand above: an edge from `a` to `b` for every ordered
  pair of distinct elements that `p` accepts, and every element a node.
- A symmetric predicate gives an edge in each direction; a program that wants
  one edge per pair writes the longhand with an ordering in `on`.
- The checker SHOULD warn when the input is a data source, since the pairs
  grow with the square of the input.

### 5. Traversal grows a graph

```hql
fn expand(g: Graph<Card, Link>, depth: Int, direction: String = "both") : HmdGraph
fn expand(xs: Collection<Card>, depth: Int, direction: String = "both") : HmdGraph
```

- `expand` MUST add every card reachable from the graph's nodes along the
  vault's authored links within `depth` steps, in `direction` (`uplinks`,
  `downlinks` or `both`), with the links among all of them.
- `depth` MUST be at least `1`, by HQL-0017.
- `expand(xs, d)` MUST equal `Graph(xs) | expand(d)`.

### 6. Reading a graph

```hql
g.nodes : Set<N>
g.edges : Set<E>
```

- A graph MUST expose its nodes and edges as sets, which settles the graph
  card's open item. They are sets because discovering a graph does not order
  it.
- `g.nodes | size`, `g.edges | filter(…)` and
  `Graph(g.nodes, g.edges | filter(…))` read and rebuild a graph with no graph
  operation of their own.

### 7. The knowledge graph

- `KnowledgeGraph` MUST keep its constructors in the knowledge extension, by
  HQL-0018: `KnowledgeGraph(cards)` reads concept cards as nodes and relation
  cards as relations, keeping each relation's identity, provenance and
  evidence.
- A `KnowledgeGraph` is a `Graph<Concept, Relation>`, so sections 5 and 6
  apply to it, and a program MAY build a plain `Graph` from its nodes and
  edges, which drops the knowledge contract and keeps the structure.

### 8. The vault's own graph

```hql
vault.cards : Seq<Card>
vault.graph : KnowledgeGraph

cards = vault.cards
graph = vault.graph
```

- `vault` is the current vault, seeded by the host, whose type and fields
  [HQL-0020](../HQL-0020/README.md) specifies. `vault.graph` MUST be
  `KnowledgeGraph(vault.cards)`, built at most once per program.
- The standard library MUST make both bindings, so a program writes `graph`
  as it writes `cards`. They are bindings, not keywords and not operations:
  `graph | expand(1)` reads the value, and `cards | graph` is refused by
  [HQL-0018](../HQL-0018/README.md), naming `Graph`.
- The document graph of the whole vault is `Graph(cards)`; it has no binding
  of its own.

## Backwards Compatibility

Breaking in one place: `Graph(cards)` keeps only the links among the given
cards, where the `graph` step kept every link of the vault. A program that
wants the outgoing links writes `cards | uplinks | Graph`. The other
constructors, `g.nodes` and `g.edges` are new. `expand` keeps its behaviour
apart from refusing depth `0`.

## Reference Implementation

- `src/extensions/graph.rs` — the constructors; `project` keeping only induced
  edges; `expand` over a graph or cards
- `std/graph.hql` — the constructor signatures beside `Graph` and `Edge`;
  `nodes` and `edges` as fields
- `std/knowledge.hql` — `KnowledgeGraph`'s constructor
- `src/checker.rs`, `field_type` — `nodes` and `edges` on `Graph`
- `doc/wiki/hql/types/graph-type.hmd` — construction, the readable node and
  edge sets, and the projection section rewritten around `Graph(…)`
- `examples/` — a corpus case per constructor, and the bipartite join

## Test Plan

Unit tests MUST include:

- `Graph(edges)` has every endpoint as a node; `Graph(nodes, [])` keeps every
  node
- `cards | filter(…) | Graph` keeps no edge to a filtered-out card;
  `cards | uplinks | Graph` keeps them
- a relation card contributes an edge and its endpoints and is not a node
- the join form equal to its `map` and `Edge` longhand; the predicate form
  equal to its self-join longhand
- the bipartite join over a fixture vault has only bird-to-habitat edges
- `expand(xs, 1)` equal to `Graph(xs) | expand(1)`; `expand(depth = 0)`
  refused
- `g.nodes | size` and `g.edges | size` match the rendered node-link JSON

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Changelog

- 2026-10-18: drafted
