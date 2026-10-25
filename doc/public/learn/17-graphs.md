# 17. Links and graphs

A document that writes `[[frankenstein]]` links to another. The links of a
vault make its cards a **graph**: cards are the nodes, links are the edges.
This chapter follows links, builds graphs, and draws them.

## Following links

An edge goes from the card that wrote the link, its `source`, to the card it
names, its `target`. `uplinks` gives the edges leaving a card, and `downlinks`
the edges arriving at it:

```hql
@[[mrs-dalloway]] | uplinks
```

```text
[mrs-dalloway -> virginia-woolf]
```

```hql
@[[virginia-woolf]] | downlinks | map(e => e.source)
```

```text
["a-room-of-ones-own", "mrs-dalloway", "the-common-reader"]
```

A link in a body may carry data of its own, written after it in braces:
`[[bearer-tokens]] {relation: defines}`. The edge holds it as `e.data`.

## Building a graph

`Graph(cards)` — or `cards | Graph` — is the graph of those cards and the
links among them:

```hql
cards
| filter(c => c.name == "virginia-woolf" or c.metadata.author == "Virginia Woolf")
| Graph
| text
```

```text
a-room-of-ones-own
  -> virginia-woolf
mrs-dalloway
  -> virginia-woolf
the-common-reader
  -> virginia-woolf
virginia-woolf
  -> a-room-of-ones-own
  -> mrs-dalloway
  -> the-common-reader
```

A graph is a value with two fields, both sets: its `nodes`, the cards, and its
`edges`. The vault's whole graph is bound as `graph`:

```hql
{nodes: graph.nodes | size, edges: graph.edges | size}
```

```text
{edges: 22, nodes: 14}
```

A graph can also be built from edges alone, `Graph(edges)`, or from cards and
edges together, `Graph(nodes, edges)`. Its nodes are the cards given and every
edge's endpoints:

```hql
@[[virginia-woolf]] | downlinks | Graph | text
```

```text
a-room-of-ones-own
  -> virginia-woolf
mrs-dalloway
  -> virginia-woolf
the-common-reader
  -> virginia-woolf
virginia-woolf
```

## Growing a graph

`expand(depth = n)` starts from some cards and follows links outwards, `n`
steps, in both directions. Every node it reaches records **why it is
present** — which card it was reached from, and how far away:

```hql
@[[mrs-dalloway]] | expand(depth = 1) | text
```

```text
club  (expanded from mrs-dalloway at depth 1)
  -> mrs-dalloway
mrs-dalloway
  -> virginia-woolf
virginia-woolf  (expanded from mrs-dalloway at depth 1)
  -> mrs-dalloway
```

`direction = "uplinks"` or `"downlinks"` follows links one way only. A graph
of matches and their neighbours never claims that the neighbours matched: the
reason travels with each node.

## A relation is an edge

The card `frankenstein-inspires-dune` declares a relation in its header:

```markdown
---
title: Frankenstein inspires Dune
metadata:
  category: influence
  knowledge:
    type: Relation
    relation:
      source: frankenstein
      target: dune
---
```

In a graph such a card is not a node but the edge it declares, between the two
cards it names:

```hql
cards | of_type(RelationCard) | Graph | text
```

```text
dune  (expanded from frankenstein-inspires-dune at depth 1)
frankenstein  (expanded from frankenstein-inspires-dune at depth 1)
  -> dune
```

## Drawing a graph

A graph is written in a diagram format by the step named for it. `mermaid`
writes a Mermaid diagram and `d2` a D2 one; `json` writes the node-link form a
graph library reads:

```hql
cards | filter(c => c.metadata.category == "book") | Graph | mermaid
```

```text
graph LR
  n0["a-room-of-ones-own"]
  n1["dune"]
  n2["frankenstein"]
  n3["leaves-of-grass"]
  n4["mrs-dalloway"]
  n5["the-common-reader"]
  n6["the-hobbit"]
  n2 --> n1
```

In a Markdown document — [chapter 19](19-documents.md) — a graph is written as a
fenced `d2` block, which documentation sites draw.

## What is next

`expand` grew a graph from a starting card. [Chapter 18](18-retrieval.md) finds
the starting cards for a question in words.
