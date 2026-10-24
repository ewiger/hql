# HQL { .hql-hero }

HQL, the Hyper Query Language, is a statically typed language for querying,
transforming, connecting and computing over data, knowledge and AI workflows.
A program is type-checked before it is evaluated; a type error prevents
evaluation. Values flow through pipelines of typed steps — `filter`, `map`,
`group`, streams and batches, sources and sinks, retrieval, graph traversal,
agent runs — over records and lists, files and standard streams, and a vault
of Markdown and HyperMarkDown documents.

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
cards              : Seq<Card>      the vault's cards, in sorted-walk order
semantic("…")      : Ranking<Card>  cards ranked by embedding similarity to the query
take(3)            : Ranking<Card>  the three highest-ranked hits
expand(depth = 1)  : Graph          those cards, every card one link away, and the edges between them
text               : Text           the graph as lines, each node with why it is present
```

Each node of the graph stores a presence: `retrieved` with its score, or
`expanded from <seed> at depth <n>`. A reader of the output can tell a node
that matched the query from a node that was reached by traversal.

<div class="grid cards" markdown>

- **Typed functional core** — expressions, generic functions, pattern
  matching and collections, checked before execution.
- **Dataflow** — finite data, `Stream<T>` and batches in one pipeline model.
- **Sources, targets and sinks** — files, standard streams and channels, read
  and written through typed endpoints.
- **Knowledge** — a vault of documents, the graph of their links, and lexical
  and semantic retrieval with provenance.
- **AI workflows** — agents as cards, context and prompts as values, and runs
  that replay from recordings.
- **Executable Markdown** — HQL cells rendered from the CLI and run in VS Code.

</div>

## Start here

<div class="grid cards" markdown>

- **[Introduction](public/introduction.md)** — HQL through its views: a typed
  functional language, a dataflow language, a data tool, a knowledge language
  and an AI harness.
- **[Install](public/install.md)** — a release binary or `cargo install hql`.
- **[Learn HQL](public/learn/index.md)** — the book: from a first value to
  records and lists, SQL-style questions, a vault of documents, graphs,
  retrieval, streams, documents that compute themselves, reading and writing
  data, logging, and agents, one runnable program at a time.
- **[Reference](public/reference.md)** — the implemented grammar, the command
  line, and reporting.

</div>

## Extensions and sources

Retrieval, graph traversal, presentation, I/O, logging and AI are extensions:
providers of typed steps registered in the binary — `core`, `graph`, `format`,
`lexical` and `semantic` today, with `io`, `logging` and `ai` in the next
release. The same mechanism carries HQL to further sources, so that one program
queries and moves data across SQL databases such as Postgres, JSON and YAML
documents, message queues such as Kafka, and REST APIs. HQL's operations take their inspiration from jq and SQL,
among others.

## HQL and HyperMarkDown

HyperMarkDown is the document format most HQL vaults are written in. HQL does
not define or change that format; it reads
[HyperMarkDown](https://hypermarkdown.org/), plain Markdown, YAML and JSON,
and is a separate language in its own repository.
