# HQL { .hql-hero }

HQL, the Hyper Query Language, is a statically typed expression and query
language. A program is type-checked before it is evaluated; a type error
prevents evaluation. Values are transformed through pipelines of operations
such as `filter`, `map`, `sort`, retrieval and graph traversal, over a vault
of Markdown, HyperMarkDown, YAML and JSON documents.

```hql
import semantic

cards
| semantic("night hunting birds")
| take(3)
| expand(depth = 1)
| text
```

Each step consumes the previous step's value and returns a new typed value:

```text
cards              : Set<Card>      the vault's cards
semantic("…")      : Ranking<Card>  cards ranked by embedding similarity to the query
take(3)            : Ranking<Card>  the three highest-ranked hits
expand(depth = 1)  : Graph          those cards, every card one link away, and the edges between them
text               : Text           the graph as lines, each node with why it is present
```

Each node of the graph stores a presence: `retrieved` with its score, or
`expanded from <seed> at depth <n>`. A reader of the output can tell a node
that matched the query from a node that was reached by traversal.

!!! warning "In development"

    HQL is a language in design. The binary implements a statically typed
    expression core, a vault of Markdown and HyperMarkDown documents,
    pipelines over its cards, `lexical` and `semantic` retrieval, and graph
    traversal (`uplinks`, `downlinks`, `expand`). The [wiki](wiki/README.md)
    specifies more language than the binary implements, and each card states
    its implementation status.

## Start here

<div class="grid cards" markdown>

- **[Introduction](public/introduction.md)** — what HQL is, what it operates
  on, and what it does not do.
- **[Install](public/install.md)** — a release binary or `cargo install hql`.
- **[A tour of HQL](public/tour.md)** — values, collections, lambdas and
  pipelines, one runnable program at a time.
- **[Queries in documents](public/documents.md)** — a card that carries a query
  and renders its own answer.
- **[Retrieval and graphs](public/retrieval.md)** — `lexical`, `semantic`,
  `expand`, and the presence each node stores.
- **[Reference](public/reference.md)** — the implemented grammar, the command
  line, and reporting.

</div>

## Extensions and sources

Retrieval, graph traversal and presentation are extensions: providers of typed
steps registered in the binary. The implemented extensions are `core`,
`graph`, `format`, `lexical` and `semantic`. The design direction is
extensions over further sources, so that one program queries across SQL
databases such as Postgres, JSON and YAML documents, message queues such as
Kafka, and REST APIs. HQL's operations take their inspiration from jq and SQL,
among others; see [extensions](wiki/hql/extensions.md).

## HQL and HyperMarkDown

HyperMarkDown is the document format most HQL vaults are written in. HQL does
not define or change that format; it reads
[HyperMarkDown](https://hypermarkdown.org/), plain Markdown, YAML and JSON,
and is a separate language in its own repository. This site is itself a
HyperMarkDown vault: the [wiki](wiki/README.md) is the language's design, one
card per concept.
