# Retrieval and graphs

HQL ranks cards against a query and traverses the links between them. Both are
extensions: retrieval is imported where it is used, and graph steps are part of
a prelude a vault may decline.

The [birds fixture](https://github.com/ewiger/hql/tree/main/tests/fixtures/birds)
used here is fifty-six encyclopedia articles with an embedding index committed
beside them.

## Two retrievals

`lexical(query)` ranks cards by textual match with the query. It needs no
index:

```hql
import lexical

cards
| lexical("night hunting birds")
| take(3)
| map(h => h.card.name)
```

```text
[mute-swan, passeriformes, alcedinidae]
```

`semantic(query)` compares an embedding of the query with the embeddings in
the vault's index and returns cards ranked by similarity:

```hql
import semantic

cards
| semantic("night hunting birds")
| take(3)
```

That returns the owls, although not one of their articles contains the word
"night". The index is built ahead of time by
[`contrib/semantics/`](https://github.com/ewiger/hql/tree/main/contrib/semantics),
and `hql` reads it; the binary never loads the model. The vault's `hql.toml`
names the index and the embedder that filled it.

## Retrieval provenance

A retrieval returns a `Ranking<Card>`. Each hit carries its card, its score,
and the retrieval that produced it: query, index, model, model revision,
metric, and whether the search was approximate.

`take` requires elements with an ordering key of their own. A card has one, so
a prefix is reproducible independently of the vault's file order.

## Graph traversal and node presence

`uplinks` and `downlinks` give the edges leaving and entering a card. `expand`
traverses outwards and returns a graph, and `graph` projects a collection into
one without traversing:

```hql
import lexical

cards
| lexical("owl")
| take(2)
| expand(depth = 1)
| table
```

```text
node          presence
------------  ----------------------------------------------
barn-owl      expanded from barn-owl-in-tytonidae at depth 1
strigidae     expanded from tawny-owl at depth 1
strigiformes  expanded from barn-owl at depth 1
tawny-owl     retrieved 0.2483
tytonidae     expanded from barn-owl-in-tytonidae at depth 1
```

Each node stores a presence: `retrieved` with its score for a hit, `expanded
from <seed> at depth <n>` for a node reached by traversal, and `member` for a
node that `graph` projected from a collection. The presence distinguishes a
node that matched the query from a node that was reached from one.

See [semantic search](../wiki/hql/semantic-search.hmd) and
[the graph type](../wiki/hql/types/graph-type.hmd) for the design.
