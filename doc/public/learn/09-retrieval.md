# 9. Finding cards by meaning

`filter` answers a question with a yes or no per card. Retrieval answers a
question in words with a **ranking**: every card that matches, best first,
each with a score. HQL has two retrievals, each an extension a program
imports.

## Lexical retrieval

`lexical` matches the words of a query against the words of each card. It
needs nothing built in advance:

```hql
import lexical
cards | lexical("desert spice") | take(3)
```

```text
[dune (0.2928), frankenstein-inspires-dune (0.2778), frankenstein (0.2471)]
```

`import lexical` makes the extension's step available to the program. Without
it, the name is not a step, and the error says which import provides it:

```hql
cards | lexical("desert spice")
```

```text
check error[unknown-name]: `lexical` is provided by the `lexical` extension: write `import lexical`
 --> <expression>:1:9
  |
1 | cards | lexical("desert spice")
  |         ^^^^^^^^^^^^^^^^^^^^^^^
```

## A ranking holds hits

A ranking is a sequence of **hits**. A hit holds the card it retrieved, as
`value`, and its `score`:

```hql
import lexical
cards
| lexical("desert spice")
| take(2)
| map(h => {card: h.value.name, score: h.score})
```

```text
[{card: "dune", score: 0.2927674902397191}, {card: "frankenstein-inspires-dune", score: 0.2777641653663699}]
```

A score is evidence that a card is relevant to the query, not relevance
itself, so every hit carries the retrieval that produced it — the model, the
index, the metric, and whether the search was approximate:

```hql
import lexical
ranked = cards | lexical("desert")
{model: ranked.retrieval.model, metric: ranked.retrieval.metric, approximate: ranked.retrieval.approximate}
```

```text
{approximate: false, metric: "cosine", model: "hql.hashbag.v1"}
```

`take` keeps a ranking a ranking, in the retrieval's order. Any other order is
the program's own: sorting a ranking by another key makes it a plain sequence
of hits.

## Retrieval, then the graph

A ranking is a collection of cards, so every step of the earlier chapters
applies. `expand` grows a graph from the best hits, and the nodes it adds say
they were reached, not retrieved:

```hql
import lexical
cards
| lexical("a creature and its maker")
| take(2)
| expand(depth = 1)
| text
```

```text
club  (expanded from frankenstein at depth 1)
  -> dune
  -> frankenstein
  -> leaves-of-grass
dune  (expanded from frankenstein at depth 1)
frankenstein  (retrieved 0.2064)
  -> mary-shelley
  -> dune
leaves-of-grass  (expanded from walt-whitman at depth 1)
  -> walt-whitman
mary-shelley  (expanded from frankenstein at depth 1)
  -> frankenstein
walt-whitman  (retrieved 0.1261)
  -> leaves-of-grass
```

## Semantic retrieval

`lexical` matches spellings: a card that never uses the query's words scores
nothing. `semantic` scores by meaning, against embeddings a language model
computed ahead of time into an index the vault names in `hql.toml`. The
tutorial vault has no index; the birds corpus in `tests/fixtures/birds` has
one. Asked for night-hunting birds, `semantic` finds the owls, though no owl
article contains the word "night":

<!-- run: --vault tests/fixtures/birds -->
```hql
import semantic
cards | semantic("night hunting birds") | take(3) | map(h => h.value.title)
```

```text
["Great horned owl", "Barn owl", "Strigidae"]
```

The same question, by spelling:

<!-- run: --vault tests/fixtures/birds -->
```hql
import lexical
cards | lexical("night hunting birds") | take(3) | map(h => h.value.title)
```

```text
["Mute swan", "Passeriformes", "Alcedinidae"]
```

The binary never loads a model: [`contrib/semantics/`](https://github.com/ewiger/hql/tree/main/contrib/semantics)
builds the index, and `hql doctor` checks the chain — vault, configuration,
index, model, query vectors — and names the first part that is missing.

## What is next

Every collection so far ended. [Chapter 10](10-streams.md) meets one that does
not.
