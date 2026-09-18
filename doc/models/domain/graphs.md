# The graph domain

Status: preferred conceptual model; no graph engine, traversal vocabulary or
renderer exists. The *implementation* is a bootstrap evaluator over Int, Float,
Bool and same-type addition. Type syntax here is illustrative.

This document defines one of the two domains HQL reasons in. The other is the
[knowledge model](knowledge.md). They are peers: neither is a layer of the
other, and a type in one is not a supertype of a type in the other — the rule
that [type system](../../wiki/hql/type-system.hmd) states. The graph domain is
the structural one. It knows
nodes, edges and adjacency, and it knows nothing about meaning, truth or
evidence.

## Vocabulary

```text
Node        whatever a graph relates; never a type of its own
Edge[S,T]   a directed connection: source, target, data
Graph[N,E]  a set of nodes and a set of edges over them
Link[S,T]   an edge in a document graph
Relation    a knowledge relation, which is also an edge
```

There is no `Node` type. What a node is arrives as a type parameter, so a graph
over documents and a graph over concepts share one definition rather than a node
union or a `kind` field.

```hql
type Edge[S, T] {
    source: S
    target: T
    data:   Data
}

type Graph[N, E]
```

`Edge[S, T]` types the two ends separately. An edge whose ends differ in kind —
a card pointing at a concept, a concept supported by a document — needs no union
at the node position, which is what keeps a mixed graph expressible.

The payload is [`Data`](../../wiki/hql/types/data-type.hmd): a tree whose keys
belong to whoever wrote them. `relation: parent` is an entry, not grammar.
Nothing in the core knows what it means, and an edge annotated that way is not
thereby a `Parent` edge — an authored key never mints a type.

## Two instantiations

```hql
type Link[S, T]     <: Edge[S, T]
type HmdGraph       <: Graph[Card, Link]

type Relation[S, T] <: Edge[S, T]
type KnowledgeGraph <: {Knowledge, Graph[Concept, Relation]}
```

The node parameters differ in a way worth reading carefully. The document graph
is over `Card`, because every card is a document and every document is a node in
it — a relation card included, since structurally it is a file with links like
any other. The knowledge graph is over `Concept`, so only a `ConceptCard`
qualifies: `ConceptCard <: {Card, Concept}` supplies the node, while a
`RelationCard` supplies an edge and no node. The same file therefore sits at
different positions in the two graphs, which is exactly what it means for the
domains to be peers.

| Graph | Nodes | Edges | Where it comes from |
| --- | --- | --- | --- |
| Document graph | documents | authored links | the vault's own structure |
| Knowledge graph | concepts | relations | interpretation, performed and retained |

A document graph is discovered: the links are already written, and reading them
asserts nothing. A knowledge graph is argued for, and the rule used to derive it
must be kept with the result. That difference is why only the second declaration
carries a knowledge parent: a graph of concepts and relations that had dropped
its conflicting claims, provenance and evidence would be a picture of knowledge
rather than knowledge, and the type says so.

Both render through the same machinery without being the same object, which is
the reason `Graph` is parameterized rather than declared once with a
discriminator.

`Relation[S, T] <: Edge[S, T]` is an ordinary narrowing: an edge has source,
target and data; a relation has those and adds identity, evidence and
provenance. What is *not* true is `Relation <: Card`. A relation may be
imported, derived or read from a store with no document anywhere; a
`RelationCard` merely represents one. See [knowledge model](knowledge.md).

## Where edges come from

**Authored links.** The [link operator](../../wiki/hql/operators/link-op.hmd)
writes one:

```hql
[[alice]] -> [[bob]] { relation: parent }
```

Inside `alice`, the short form `[[bob]] { ... }` means the same thing, because
the containing document supplies the source. Authoring it adds the edge to the
vault's graph; authoring it again updates the annotation rather than adding a
second edge.

**Extensions.** A database, a graph store or an indexer can supply edges the
vault never wrote. The graph domain does not care which, and an extension
supplying edges is the same kind of participant as one supplying documents.

**Derivation.** A rule can produce edges from existing ones. The rule is part of
the result and must survive with it, which is a knowledge concern rather than a
structural one.

## Identity

The two graphs differ here, and the difference is not cosmetic.

**A document edge is identified by its endpoints.** One `alice -> bob` edge
exists; writing the link twice addresses it twice and merges the annotation. The
document graph is therefore a simple graph, which is right for it: two links to
`bob` in `alice` are one dependency stated twice, not two dependencies.

**A knowledge edge is identified by itself.** Distinct relations may make
conflicting claims about the same pair, and knowledge must retain both with
their provenance and evidence rather than merging them into one annotated edge.
The knowledge graph is a multigraph, and the identity is the relation's own —
which is exactly what `Relation` adds to `Edge`.

[Collections](../../wiki/hql/collections.hmd) already require that set
membership must not equate distinct claims merely because they share endpoints.
This is that requirement with a type behind it.

Node identity is open. Two documents, two concepts, or a document and the
concept it describes: when these are the same node has not been decided, and
`Set[N]` presumes an answer.

## Direction and traversal

An edge is directed. `uplinks` are the outgoing direction and `downlinks` the
incoming one, and they traverse one edge set rather than two: a document cannot
know what points at it by reading itself, which is why the incoming direction
needs a vault. A node may be its own source and target.

Whether a graph exposes its parts as collections, and which ones, is open:

```hql
graph.nodes : Set[N]
graph.edges : Set[E]
```

`Set` follows the collections model, since discovering edges must not establish
an ordering guarantee. Less clear is whether a graph over a vault is
materialized at all. It may be a query a source executes — a store, a database
or the vault index — which is the execution freedom
[pipes](../../wiki/hql/types/pipes.hmd) already reserves for collection steps.
A traversal, subgraph or path vocabulary has not been designed, and whether it
is ordinary functions or a separate query surface is undecided.

## Projection and presentation

```hql
knowledge
| graph
```

`graph` is an ordinary function from a knowledge value to a graph, not a
presentation keyword and not a magical pipeline step. The value it returns happens to
have a renderer registered by the graph extension, which is what makes it appear
in a host as a picture.

Its result type is the open question. A `KnowledgeGraph` keeps both contracts,
so the projection loses nothing and the result can still be asked about claims,
provenance and evidence. A bare `Graph[Concept, Relation]` is the weaker,
genuinely lossy result, which is all a host that only draws pictures needs.
Whether these are two operations or one whose result type is stronger than it
used to be is undecided.

That separates the two readings that have been competing: the projection
produces a value, and rendering is a capability *of* that value, supplied by an
extension as `render(Graph) -> Html` rather than inherited through `<:`. The
naming is still unsettled — whether the projection and the view may share the
name `graph`, and whether `HmdGraph` should be called that when it is a graph
over cards rather than a presentation.

A projection must not silently turn an assertion into a truth or collapse
distinct claims — which is exactly what a `KnowledgeGraph` is typed to prevent.
Graph is one projection of knowledge among several; tables, lists and diagrams
are others.

## Where the domain lives

The layering is core → graph → knowledge, with document sources beside it.

- The **core** names no graph. It has values, bindings, functions and pipes.
- The **graph extension** owns `Edge`, `Graph` and their operations.
- The **knowledge extension** specializes them for meaning, and depends on the
  graph capabilities rather than the other way round.
- **HMD** is one source of documents and specializes them for the vault. Nothing
  depends on HMD.

An extension owns type knowledge and operations, not the data. This is the same
boundary the [architecture](architecture.md) draws for documents.

## Open questions

- **Variance.** `Card <: Doc`, so does `Graph[Card, Link] <: Graph[Doc, Edge]`?
  A function written over `Graph[Doc, Edge]` is useless if not. Covariance is
  defensible while graphs are immutable values and unsound once one can be
  extended in place, so the answer follows from whether construction is pure.
- **Endpoints as values or references.** A forward link to a document that does
  not exist is permitted, which argues that an edge holds references the graph
  never resolves. What a reference yields when the target is missing is the open
  question the link operator records.
- **Horizon.** `cards` stops at the vault. A graph built from imported
  namespaces or an external store may not, and what bounds it is undecided.
- **Merge and conflict policy** when two sources supply the same edge, and
  whether annotation merging is per key or deep.
- **Whether `Graph` is one type or a family** with a shared surface.
- **Weights, hyperedges and ordered adjacency** are not part of the model and
  have not been argued for or against.

See [Graph](../../wiki/hql/types/graph-type.hmd) for the type card,
[link operator](../../wiki/hql/operators/link-op.hmd) for the edge-writing
syntax, [type system](../../wiki/hql/type-system.hmd) for `<:`, generics and
refinement, [knowledge model](knowledge.md) for the other domain,
[semantic search](semantic-search.md) for the first concrete demand on a
traversal vocabulary, and the
[design conversation](../../conversations/sets-n-subtypes.md) this model was
drawn from.
