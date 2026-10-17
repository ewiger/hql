# HQL { .hql-hero }

HQL — the Hyper Query Language — is a typed functional language for querying
and computing over structured knowledge: documents, cards, metadata and the
graph their links make.

```hql
import semantic

cards
| semantic("night hunting birds")
| take(3)
| expand(depth = 1)
| table
```

That program ranks a vault of encyclopedia articles by meaning, keeps the three
best matches, walks one link outwards from each, and prints the graph. Every
step is checked before anything runs, and every node in the answer records
*why* it is there.

!!! warning "In development"

    HQL is a language in design. What runs today is a typed expression core, a
    vault of Markdown and HyperMarkDown documents, pipelines over its cards, two
    named retrievals and graph traversal. The [wiki](wiki/README.md) specifies
    more language than the binary implements, and says so card by card.

## Start here

<div class="grid cards" markdown>

- **[Introduction](public/introduction.md)** — what HQL is for, and what it is
  deliberately not.
- **[Install](public/install.md)** — a release binary or `cargo install hql`.
- **[A tour of HQL](public/tour.md)** — values, collections, lambdas and
  pipelines, one runnable program at a time.
- **[Queries in documents](public/documents.md)** — a card that carries a query
  and renders its own answer.
- **[Retrieval and graphs](public/retrieval.md)** — `lexical`, `semantic`, and
  traversal that keeps its evidence.
- **[Reference](public/reference.md)** — the implemented grammar, the command
  line, and reporting.

</div>

## HQL and HyperMarkDown

HQL is developed alongside [HyperMarkDown](https://hypermarkdown.org/), the
strict markdown dialect its vaults are usually written in. HyperMarkDown is the
authored representation; HQL is a separate language that reads it, and reads
plain Markdown, YAML and JSON as well. This site is itself a HyperMarkDown
vault: the [wiki](wiki/README.md) is the language's design, rendered card by
card.
