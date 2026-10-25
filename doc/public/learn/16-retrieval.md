# 16. Finding cards by meaning

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
name error: `lexical` is provided by the `lexical` extension: write `import lexical`
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
nothing, and a card that uses common ones scores something. `semantic` scores
by meaning, against **embeddings** a language model computed ahead of time
into an index the vault names in `hql.toml`:

```toml
[semantics]
index = ".hql/index.sqlite"
```

Asked for a creature and its maker, `semantic` ranks Frankenstein, the
relation card that names it, and Mary Shelley, though none of the three
contains the word "creature":

```hql
import semantic
cards | semantic("a creature and its maker") | take(3) | map(h => h.value.title)
```

```text
["Frankenstein", "Frankenstein inspires Dune", "Mary Shelley"]
```

The same question, by spelling, finds Frankenstein and then two cards that
share only its small words:

```hql
import lexical
cards | lexical("a creature and its maker") | take(3) | map(h => h.value.title)
```

```text
["Frankenstein", "Walt Whitman", "Librarian"]
```

A semantic ranking reports the model that embedded the cards, so two indexes
built by two models are never compared by score:

```hql
import semantic
ranked = cards | semantic("a creature and its maker")
{model: ranked.retrieval.model, metric: ranked.retrieval.metric, approximate: ranked.retrieval.approximate}
```

```text
{approximate: false, metric: "cosine", model: "sentence-transformers/all-MiniLM-L6-v2"}
```

The binary never loads a model: [`contrib/semantics/`](https://github.com/ewiger/hql/tree/main/contrib/semantics)
embeds the cards and builds the index, and embeds each question the index
will be asked, named at build time with `--query`. The tutorial vault's index
holds the three questions this book asks. A question it does not hold is a
diagnostic naming the build command, not a ranking of nothing:

```hql
import semantic
cards | semantic("night hunting birds") | take(1)
```

```text
evaluation failure: the index holds no vector for `night hunting birds`, and this binary has no model to make one: rerun `hql-semantics build --query "night hunting birds"`
 --> <expression>:2:9
  |
2 | cards | semantic("night hunting birds") | take(1)
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

`hql doctor` checks the chain — vault, configuration, index, model, query
vectors — and names the first part that is missing.

## What is next

A ranking, a graph and a reading list are values a program prints.
[Chapter 17](17-documents.md) writes them into the document that asks for
them, and builds the document from its queries.
