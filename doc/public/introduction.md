# Introduction

HQL is a statically typed expression and query language. A program is a
sequence of bindings, declarations and expressions; its value is the last
expression, and its type is checked before anything is evaluated.

Querying is one thing it does. The language builds values, defines and applies
functions, reshapes data and returns typed results — a direction closer to
Kusto, Scala or Haskell than to SQL. Its operations take their inspiration from
`jq` and SQL, among others.

## What it operates on

HQL works on values: scalars, records and lists of the shape JSON and YAML
have, and typed collections — lists, sets, maps and streams. Most programs run
over a **vault**, a directory of Markdown and HyperMarkDown documents. Each
document is a **card**, and `cards` is the sequence of the vault's cards, in
the order a sorted walk of the folder reaches them.

A card has a name, a title, a path, a body, and its header's metadata. The
links between cards form a graph, and HQL follows them in either direction,
builds graphs from them, and draws them.

Retrieval, graph traversal and presentation are extensions: providers of typed
steps registered in the binary. The implemented extensions are `core`,
`graph`, `format`, `lexical` and `semantic`. The design direction is
extensions over further sources, so that one program queries across SQL
databases such as Postgres, JSON and YAML files, message queues such as Kafka,
and REST APIs.

## Design commitments

- **Static type checking.** The whole program is type-checked before it runs.
  A type error is reported where it occurs, and nothing is evaluated.
- **Pipelines of ordinary functions.** `xs | f(a)` is `f(xs, a)`. There are no
  SQL clauses: filtering, sorting, grouping and projection are functions like
  any other, and each step must accept the type the step before it produced.
- **Functions are values.** Functions are declared, passed, returned, made
  generic over their types, and defined as aliases whose types the checker
  writes.
- **Finite and endless are different types.** A `Stream<T>` need not end, and
  a step that needs its whole input refuses one before the program runs.
- **Evidence travels with results.** A retrieved card carries the retrieval
  that produced it — model, index, metric — and a node of a graph records why
  it is present.
- **Formats are types.** A value is written as `Text`, `Markdown`, `Json`,
  `Yaml`, `Csv`, `D2` or `Mermaid` by a step, and the result is text of that
  type, which a later step may still read.
- **Documents compute themselves.** The HQL blocks of a document are cells that
  share values by name; `hql render` builds the document they describe.
- **Implementation status.** Each design card states whether the binary
  implements it; the [design status](../wiki/hql/design-status.hmd) card
  collects them. A program that uses unimplemented language fails to check.

## Where to go next

[Install](install.md) the binary, then read [the HQL book](learn/index.md),
which teaches the whole language, chapter by chapter.
