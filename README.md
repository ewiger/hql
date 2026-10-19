<div align="center">

<img src="https://raw.githubusercontent.com/ewiger/hql/main/doc/assets/logo.svg" width="76" height="76" alt="">

# HQL

**Statically typed expression and query language**

[![Crates.io](https://img.shields.io/crates/v/hql.svg)](https://crates.io/crates/hql)
[![MSRV](https://img.shields.io/crates/msrv/hql?color=orange&label=rust)](https://github.com/ewiger/hql/blob/main/Cargo.toml)
[![CI](https://github.com/ewiger/hql/actions/workflows/ci.yml/badge.svg)](https://github.com/ewiger/hql/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/docs-hql.hypermarkdown.org-ffb300)](https://hql.hypermarkdown.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/ewiger/hql/blob/main/LICENSE)

</div>

**HQL — Hyper Query Language — is a statically typed language for querying, transforming, connecting and computing over data, knowledge and AI workflows.**

HQL combines functional expressions, data pipelines, streams, structured documents, semantic retrieval and knowledge graphs in one language.

```hql
birds =
    cards
    | filter(c => c.metadata.category == "bird")
    | sort(by = c => c.title)

birds
| map(c => c.title)
```

The same language can query a knowledge vault, transform JSON from the command line, build an ETL pipeline, traverse a graph, execute queries inside Markdown, or assemble context for an AI agent.

## TL;DR

- **Typed functional query language** — expressions, functions, pattern matching, collections and pipelines, checked before execution.
- **Dataflow language** — compose transformations over finite data, streams and batches.
- **Data integration and ETL** — connect files, standard streams, databases, queues and other sources and targets through one query model.
- **AI-native language** — retrieval, context construction and agent workflows are expressed as transparent data pipelines.
- **Executable Markdown** — run HQL cells inside HyperMarkDown documents and directly in VS Code, turning Markdown into live queries, reports, views and AI-enabled knowledge artifacts.
- **Knowledge graph language** — retrieve, traverse, expand and combine structured knowledge and relationships.
- **Command-line data tool** — process JSON, YAML, CSV, text and streams in the role normally filled by tools such as `jq`, while retaining types, functions and reusable programs.
- **Streaming by design** — `Stream<T>` is a first-class type; pipelines can process values as they arrive instead of materializing an entire dataset.
- **One language across execution surfaces** — use HQL from the CLI, `.hql` programs, executable documents and interactive editor cells.

Documentation: [hql.hypermarkdown.org](https://hql.hypermarkdown.org/)

---

## One language, several useful views

HQL is intentionally difficult to reduce to “a query language for X”.

It is better understood through several complementary views.

---

## A typed functional query language

HQL programs are expressions.

Values flow through pipelines, while functions, bindings, collections and pattern matching provide the tools needed for reusable computation.

```hql
fn pages(b: Data) : Int =
    match b.pages {
        some(Int n) => n,
        _ => 0
    }

large_books =
    books
    | filter(b => pages(b) > 500)
    | sort(by = b => b.title)
```

Types are checked before execution.

The type system includes collection types such as:

```text
Seq<T>
List<T>
Set<T>
Map<K, V>
```

first-class streams:

```text
Stream<T>
```

and algebraic value types such as:

```text
Option<T>
```

alongside `Data` for dynamically structured values.

Pipelines are ordinary typed expressions rather than a separate query syntax:

```hql
cards
| filter(c => c.metadata.status == "open")
| map(c => c.title)
| sort
```

This makes the same language suitable for small queries and larger programs.

---

## A dataflow language

HQL models computation as data moving through transformations.

```hql
events
| filter(e => e.kind == "purchase")
| map(e => {
    customer: e.customer,
    amount: e.amount,
})
| collect(100)
| process_batch
```

A pipeline can operate over finite collections or over streams that continue producing values.

```text
Stream<Event>
```

can become batches:

```hql
events
| collect(100)
```

producing:

```text
Stream<Seq<Event>>
```

while:

```hql
events
| collect
```

materializes a completed stream as:

```text
Seq<Event>
```

and:

```hql
items
| emit
```

turns a finite collection back into a stream.

Branches are part of the same model:

```hql
events
| tee errors =
    filter(e => e.level == "error")
| aggregate(...)
```

Dataflow is therefore not an execution detail hidden underneath HQL.

It is part of how HQL programs are written.

---

## Sources, streams, targets and sinks

External data enters HQL through typed sources.

```hql
source(stdin, Json)
```

For example:

```bash
curl https://example.org/events |
    hql 'source(stdin, Json) | filter(e => e.active)'
```

`stdin` is a host-provided endpoint. It does not need to be configured.

Files use the same model:

```hql
birds =
    source(file("birds.json"), Json)
```

as do configured sources:

```hql
warehouse =
    source("warehouse")
```

A source can stand directly in a pipeline:

```hql
source("warehouse")
| filter(r => r.created_at > cutoff)
| map(r => r.customer_id)
```

Output follows the complementary model.

A completed value can be written to a target:

```hql
report
| target(file("report.yaml"), Yaml)
```

while a stream can be consumed incrementally by a sink:

```hql
events
| sink(stdout, Json)
```

Standard endpoints such as `stdout` and `stderr` are host-provided just like `stdin`:

```hql
result
| target(stdout, Yaml)

errors
| sink(stderr, Json)
```

They do not need entries in `hql.toml`.

The distinction between whole values, streams and batches lives in the type system:

```text
Event
Seq<Event>
Stream<Event>
Stream<Seq<Event>>
```

not in separate I/O APIs.

For example:

```hql
events
| collect(100)
| sink(stdout, Json)
```

streams batches:

```text
Stream<Seq<Event>>
```

while:

```hql
events
| collect
| target(file("events.json"), Json)
```

materializes the complete stream and writes one finite result.

Formats remain orthogonal:

```text
Json
Yaml
Csv
Text
Markdown
Hql
D2
Mermaid
```

The format describes representation.

The HQL type describes the shape of the data.

---

## HQL as an ETL and integration language

Sources, transformations, streams, batches and targets all use the same expression model.

That makes HQL suitable for ETL and data integration without introducing a separate workflow language around the query language.

```hql
source("orders")
| filter(o => o.status == "completed")
| map(o => {
    customer: o.customer_id,
    total: o.total,
})
| collect(500)
| sink("analytics")
```

A pipeline can combine multiple sources:

```hql
customers = source("customers")
orders = source("orders")

...
```

or move between files and structured output:

```hql
source(file("sales.csv"), Csv)
| filter(row => row.amount > 1000)
| target(file("large-sales.json"), Json)
```

It can also mix live and finite processing:

```hql
source(stdin, Json)
| filter(e => e.level == "error")
| tee archive =
    sink(file("errors.jsonl"), Json)
| map(e => e.message)
| sink(stderr, Text)
```

The query is the dataflow.

There is no second orchestration syntax wrapped around it.

---

## Streaming by design

Streams are first-class values.

```text
Stream<T>
```

represents values produced incrementally and may be bounded or unbounded.

A stream can be transformed directly:

```hql
source(stdin, Json)
| filter(e => e.kind == "event")
| map(e => e.payload)
```

collected completely:

```hql
events
| collect
```

batched:

```hql
events
| collect(100)
```

or created from a finite collection:

```hql
items
| emit
```

This makes streaming a normal part of the type system rather than a separate execution mode.

The same operations compose over:

```text
Seq<T>
Stream<T>
Stream<Seq<T>>
```

and preserve the distinction between finite data, individual stream elements and streaming batches.

---

## An AI-native language

AI applications spend much of their complexity not in the model call itself, but in everything around it:

- retrieving relevant information,
- expanding context,
- selecting evidence,
- constructing prompts,
- invoking agents,
- evaluating results,
- reacting to events,
- recording outputs,
- and feeding results into subsequent computation.

These are dataflow problems.

HQL expresses them as dataflow.

```hql
context =
    cards
    | semantic(question)
    | take(10)
    | expand(depth = 1)

context
| ...
```

Retrieval is not a terminal operation.

The result remains ordinary typed data that can continue through filtering, graph traversal, transformation, aggregation and rendering.

```hql
cards
| semantic("distributed execution")
| take(20)
| filter(hit => hit.score > threshold)
| expand(depth = 2)
```

This makes HQL a natural language for AI harnesses: retrieval, context construction, prompts, agents and their outputs can be represented as inspectable programs instead of opaque framework callbacks.

In HyperMarkDown, agent definitions and instructions can themselves be documents.

That means the knowledge, the prompt and the logic assembling the prompt can all remain visible, editable and queryable.

---

## Executable Markdown

HQL can run directly inside Markdown and HyperMarkDown documents.

A fenced HQL cell is both documentation and executable logic:

````markdown
# Open architecture decisions

```hql
cards
| filter(c => c.metadata.kind == "adr")
| filter(c => c.metadata.status == "open")
| sort(by = c => c.title)
```
````

The same document can be rendered from the command line:

```bash
hql render architecture.hmd
```

or used interactively in VS Code through the HyperMarkDown extension:

[HyperMarkDown VS Code extension](https://github.com/ewiger/hypermarkdown/tree/main/tools/hmd-vsc-ext)

The extension supports HQL cells directly in HyperMarkDown documents, making the document itself an interactive query surface.

This changes the role of Markdown.

A document can become:

- a live project index,
- an architecture or ADR view,
- a generated table or report,
- a knowledge-base query,
- a semantic-search surface,
- a project dashboard,
- an AI prompt,
- a context-assembly document,
- or an operational tool placed next to the prose that explains it.

For example:

````markdown
# Services

The table below is derived from the project knowledge base.

```hql
cards
| filter(c => c.metadata.kind == "service")
| sort(by = c => c.title)
| map(c => {
    service: c.title,
    owner: c.metadata.owner,
    status: c.metadata.status,
})
```
````

The query remains visible next to the text it supports.

There is no separate dashboard definition, template language or application layer required to derive the view.

The Markdown document itself becomes the view, while HQL supplies its live data.

This gives HQL several equivalent execution surfaces:

```text
CLI              hql '...'
program          query.hql
document         ```hql ... ```
interactive doc  VS Code HQL cell
```

The language remains the same across all of them.

HyperMarkDown is therefore not merely a storage format for HQL.

It is also one of HQL's natural interactive hosts.

---

## Knowledge graphs

HQL treats links and relationships as queryable structure.

Retrieval can find an initial set of nodes:

```hql
cards
| semantic("night hunting birds")
| take(3)
```

and graph traversal can expand from them:

```hql
cards
| semantic("night hunting birds")
| take(3)
| expand(depth = 1)
```

Graph operations compose with ordinary filtering, mapping, aggregation and rendering.

This allows HQL to move naturally between several forms of knowledge:

```text
documents
    ↓
retrieval
    ↓
concepts and relationships
    ↓
graph traversal
    ↓
structured result
```

A knowledge graph is therefore not a separate database API attached to the language.

It is another structure that participates in HQL pipelines.

---

## Semantic and lexical retrieval

HQL provides retrieval as a first-class pipeline operation.

Lexical search:

```hql
cards
| lexical("night hunting birds")
| take(3)
```

Semantic search:

```hql
cards
| semantic("night hunting birds")
| take(3)
```

The result can immediately participate in further computation:

```hql
cards
| semantic(question)
| take(20)
| filter(hit => hit.score > threshold)
| expand(depth = 2)
```

This matters especially for retrieval-augmented AI workflows, where search is usually only the first stage of context construction.

Retrieval results retain their provenance, so the result does not need to lose information about how it was obtained when it continues through a pipeline.

---

## A command-line data Swiss army knife

HQL is useful without a vault or project.

Pretty-print JSON from standard input:

```bash
cat data.json |
    hql 'source(stdin, Json)' --to json
```

Filter a JSON stream:

```bash
cat birds.jsonl |
    hql '
        source(stdin, Json)
        | filter(b => b.count > 2)
        | map(b => b.species)
    '
```

Convert JSON to YAML:

```bash
cat config.json |
    hql 'source(stdin, Json)' --to yaml
```

Work with files:

```hql
source(file("birds.json"), Json)
| filter(b => b.count > 2)
| target(file("popular-birds.yaml"), Yaml)
```

Or move the query into a reusable program:

```bash
hql run report.hql
```

HQL occupies some of the same space as `jq`, but its scope is deliberately broader:

```text
jq-style structured data processing
+ static types
+ functions
+ reusable programs
+ streams
+ batching
+ multiple sources and targets
+ ETL and dataflow
+ knowledge graphs
+ semantic retrieval
+ executable Markdown
+ AI workflows
```

The goal is not to imitate the syntax of existing tools.

It is to provide one coherent language across these workloads.

---

## Formats are types

HQL treats data representations explicitly.

```hql
format(result, Json)
format(result, Yaml)
format(result, Markdown)
```

Presentation types include:

```text
Text
Json
Yaml
Csv
Markdown
Hql
D2
Mermaid
```

Formats describe encoding rather than execution shape.

For example:

```hql
items : Seq<Item>

items
| target(stdout, Json)
```

produces one JSON array.

But:

```hql
items
| emit
| sink(stdout, Json)
```

writes items incrementally as individual JSON documents.

Likewise:

```hql
events
| collect(100)
| sink(stdout, Json)
```

writes a stream of JSON arrays.

The difference comes from:

```text
Seq<Item>
Stream<Item>
Stream<Seq<Item>>
```

not from introducing separate formats such as a special JSON-lines data type.

---

## HyperMarkDown

HyperMarkDown is a natural companion to HQL, but HQL is not limited to HyperMarkDown.

A HyperMarkDown vault provides structured cards, metadata, links and knowledge relationships:

```hql
cards
| filter(c => c.metadata.kind == "adr")
| map(c => c.title)
```

The same language can operate over other sources and can be used without a vault at all.

This distinction is intentional:

**HyperMarkDown provides structured knowledge. HQL computes over it.**

Together they also provide a natural programmable-document environment: HyperMarkDown supplies the document structure, while HQL cells derive views, queries and computation directly inside those documents.

---

## Example: from knowledge to generated documentation

Suppose a project stores architecture decisions as cards.

A document can derive its architecture index:

````markdown
# Architecture decisions

```hql
cards
| filter(c => c.metadata.kind == "adr")
| sort(by = c => c.metadata.date)
| map(c => {
    title: c.title,
    status: c.metadata.status,
})
```
````

The same knowledge can be searched semantically:

```hql
cards
| semantic("decisions affecting event delivery")
| take(10)
```

expanded through relationships:

```hql
cards
| semantic("event delivery")
| take(5)
| expand(depth = 2)
```

and reused as context by an AI workflow.

The document, query, graph and AI context all operate over the same underlying model.

---

## Example: one language across data and knowledge

A typical HQL program does not need to care whether every input came from the same kind of system.

```hql
adrs =
    cards
    | filter(c => c.metadata.kind == "adr")

metrics =
    source(file("metrics.csv"), Csv)

events =
    source("deployment-events")

...
```

Each source can participate in ordinary HQL computation.

This is the larger idea behind the language:

```text
files
stdin
vaults
databases
queues
network sources
knowledge graphs
AI retrieval
```

all become inputs to the same typed dataflow model.

---

## Effects and external I/O

HQL distinguishes pure computation from effectful interaction with external systems.

Constructing an endpoint does not itself perform I/O:

```hql
file("report.json")
```

Actual effects occur when the endpoint is used:

```hql
source(file("report.json"), Json)
```

or:

```hql
report
| target(file("report.json"), Json)
```

The execution host controls which effects are permitted.

This matters especially for executable documents.

A host such as:

```bash
hql render document.hmd
```

can allow document-local reads while refusing filesystem writes or network access.

The same language can therefore run under different capability policies without changing the program model.

---

## CLI

The CLI exposes HQL directly without introducing a second language of command-line switches.

```text
hql eval <program>     check and evaluate an expression
hql run <file>         check and evaluate an HQL program
hql check <file>       type-check without evaluating
hql repl               interactive HQL session
hql render <file>      evaluate HQL cells in a document
hql builtins           inspect available operations
hql config             inspect active configuration
hql doctor             diagnose the configured environment
```

Choose the presentation format of the final result with:

```bash
hql run query.hql --to json
```

or:

```bash
hql run query.hql --to markdown
```

Input and output semantics stay in the language:

```hql
source(stdin, Json)

source(file("input.csv"), Csv)

result
| target(file("result.yaml"), Yaml)

stream
| sink(stdout, Json)
```

while:

```text
--to
```

only controls how the CLI renders the program's final result.

---

## Install

Download the appropriate release from:

[GitHub Releases](https://github.com/ewiger/hql/releases)

or install with Cargo:

```bash
cargo install hql --locked
```

Install the current repository version:

```bash
cargo install --git https://github.com/ewiger/hql --locked
```

Build from a clone:

```bash
cargo install --path . --locked
```

or:

```bash
cargo build --release
```

See [CHANGELOG.md](CHANGELOG.md) for changes between releases.

---

## Learn HQL

The full language documentation is at:

**[hql.hypermarkdown.org](https://hql.hypermarkdown.org/)**

A good way to begin is with a pipeline:

```hql
cards
| filter(c => c.metadata.status == "open")
| map(c => c.title)
| sort
```

and then move outward into:

- bindings and expressions,
- collections and types,
- functions and pattern matching,
- streams and batching,
- sources, targets and sinks,
- retrieval,
- knowledge graphs,
- executable documents,
- and AI workflows.

---

## Design philosophy

HQL is built around a small number of strong ideas.

### Everything is an expression

Queries compose with ordinary language constructs instead of living in a separate statement language.

### Types describe data before execution

Pipelines are checked as programs rather than discovered dynamically one operation at a time.

### Dataflow is part of the language

Finite values, streams and batches use the same pipeline model.

### Data location is orthogonal to computation

A transformation should not fundamentally change because its input moved from a file to stdin, a vault, a database or a stream.

### Knowledge is data

Documents, metadata, relationships, search results and graph nodes are all available for computation.

### AI is computation over context

Retrieval, context assembly, agents and their outputs belong in inspectable pipelines.

### Markdown can be executable without ceasing to be Markdown

HQL cells live next to the knowledge and prose they derive from and can run both from the CLI and interactively inside VS Code.

### One language should span local and larger dataflows

The same language model should remain valid whether computation is executed over a local value, a live stream, a configured data source or a larger execution backend.

---

## License

MIT — see [LICENSE](LICENSE).
