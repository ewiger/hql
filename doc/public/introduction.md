# Introduction

HQL is a statically typed expression and query language. A program is a
sequence of bindings and expressions; its value is the last expression, and
its type is determined by type checking before evaluation.

Querying is one thing it does. The language constructs values, evaluates
functions, transforms data, validates constraints and returns typed results, a
direction closer to Kusto, Scala, Haskell or Lean than to SQL. Its operations
take their inspiration from jq and SQL, among others.

## What it operates on

HQL works over a **vault**: a directory of documents. A document may be
Markdown, HyperMarkDown, YAML or JSON. A **card** is a document that represents
one piece of knowledge, whichever format it was written in, and `cards` is the
sequence of cards in the vault the program runs against.

A card carries a title, a name, metadata, sections and links. Links between
cards form a graph, and HQL traverses it in either direction.

Retrieval, graph traversal and presentation are extensions: providers of typed
steps registered in the binary. The implemented extensions are `core`,
`graph`, `format`, `lexical` and `semantic`. The design direction is
extensions over further sources, so that one program queries across SQL
databases such as Postgres, JSON and YAML documents, message queues such as
Kafka, and REST APIs.

## Design commitments

- **Static type checking.** The complete program is type-checked before
  evaluation. A type error is reported with its byte range, and the program
  is not evaluated.
- **Pipelines of ordinary functions.** `xs | f(a)` is `f(xs, a)`. There are no
  SQL clauses: filtering, sorting and projection are functions like any other.
  The output type of one step must be accepted by the next.
- **Retrieval provenance and node presence.** A hit carries the retrieval
  that produced it: query, index, model, model revision, metric, and whether
  the search was approximate. A graph built by `expand` stores, per node,
  whether it was retrieved (with its score) or expanded from a seed at a
  depth.
- **Presentation is separate.** A program returns a typed value. Rendering it
  as a table, JSON or text is a final, separate step.
- **Implementation status.** Each design card states whether the binary
  implements it; the [design status](../wiki/hql/design-status.hmd) card
  collects them. A program that uses unimplemented language fails to check.

## Where to go next

[Install](install.md) the binary, then take [the tour](tour.md).
