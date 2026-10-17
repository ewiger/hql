# Retrieval and graphs

HQL ranks cards against a query and traverses the links between them. Both are
extensions: retrieval is imported where it is used, and graph steps are part of
a prelude a vault may decline.

The [birds fixture](https://github.com/ewiger/hql/tree/main/tests/fixtures/birds)
used here is fifty-six encyclopedia articles with an embedding index committed
beside them.

## Two retrievals

`lexical` matches shared spellings, offline and with nothing to build:

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

`semantic` scores against vectors a language model computed ahead of time:

```hql
import semantic

cards
| semantic("night hunting birds")
| take(3)
```

That returns the owls, although not one of their articles contains the word
"night". The model is not in the binary: the index is produced by
[`contrib/semantics/`](https://github.com/ewiger/hql/tree/main/contrib/semantics),
and `hql` only reads it. The vault's `hql.toml` names the index and the
embedder that filled it.

## A hit keeps its evidence

A retrieval returns a `Ranking<Card>` of hits. Every hit carries the retrieval
that produced it — query, index, model, model revision, metric, and whether the
search was approximate — because a score is evidence, not relevance.

`take` needs elements that carry an order of their own. A card does, so a
prefix is reproducible without the vault's file order ever being observable.

## Walking the graph

`uplinks` and `downlinks` give the edges leaving and entering a card. `expand`
traverses outwards, and `graph` projects a collection into a graph:

```hql
import lexical

cards
| lexical("owl")
| take(2)
| expand(depth = 1)
| graph
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

Each node records *why* it is present, so a graph of two matches and their
neighbours does not claim that all of them matched.

See [semantic search](../wiki/hql/semantic-search.hmd) and
[the graph type](../wiki/hql/types/graph-type.hmd) for the design.
