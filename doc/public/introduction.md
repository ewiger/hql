# Introduction

HQL is a typed functional language for structured knowledge. A program is a
sequence of bindings and expressions; its value is the last expression, and its
type is known before it runs.

Querying is one thing it does. The language is meant to construct values,
evaluate functions, transform data, validate constraints and return typed
results — a direction closer to Kusto, Scala, Haskell or Lean than to SQL.

## What it operates on

HQL works over a **vault**: a directory of documents. A document may be
Markdown, HyperMarkDown, YAML or JSON. A **card** is a document that represents
one piece of knowledge, whichever format it was written in, and `cards` is the
set of cards in the vault the program runs against.

A card carries a title, a name, metadata, sections and links. Links between
cards form a graph, and HQL can traverse it in either direction.

## Design commitments

- **Checked before evaluated.** A program is type-checked as a whole first. A
  type error is reported with its location, and nothing runs.
- **Pipelines of ordinary functions.** `xs | f(a)` is `f(xs, a)`. There are no
  SQL clauses: filtering, sorting and projection are functions like any other.
- **Evidence, not relevance.** A retrieval returns hits that carry the query,
  index, model and metric that produced them. A graph built from them records
  why each node is present.
- **Presentation is separate.** A program returns a typed value. Rendering it
  as a table, JSON or text is a final, separate step.
- **Honest about what is built.** Where the design is ahead of the
  implementation, the [design status](../wiki/hql/design-status.hmd) says so,
  and a program that uses unbuilt language fails rather than pretending.

## Where to go next

[Install](install.md) the binary, then take [the tour](tour.md).
