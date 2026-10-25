<div align="center">

<img src="https://raw.githubusercontent.com/ewiger/hql/main/doc/public/assets/logo.svg" width="76" height="76" alt="">

# HQL

**Statically typed expression and query language**

[![Crates.io](https://img.shields.io/crates/v/hql.svg)](https://crates.io/crates/hql)
[![MSRV](https://img.shields.io/crates/msrv/hql?color=orange&label=rust)](https://github.com/ewiger/hql/blob/main/Cargo.toml)
[![CI](https://github.com/ewiger/hql/actions/workflows/ci.yml/badge.svg)](https://github.com/ewiger/hql/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/docs-hql--lang.org-ffb300)](https://hql-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/ewiger/hql/blob/main/LICENSE)

</div>

**HQL, the Hyper Query Language, is a statically typed expression and query language. It combines functional expressions with pipelines over files, streams, structured data, HyperMarkDown documents, knowledge and AI workflows. A program's types are checked before it runs, and data read from outside is checked against declared contracts where it enters.**

HQL combines functional expressions, data pipelines, streams, structured documents, semantic retrieval and knowledge graphs in one language.

```hql
type Book { title : String, year : Int, rating : Int }

books = [
    Book(title = "Dune", year = 1965, rating = 5),
    Book(title = "Emma", year = 1815, rating = 3),
    Book(title = "Beloved", year = 1987, rating = 4),
]

books
| filter(b => b.rating >= 4)
| sort(b => b.title)
| map(b => b.title)
```

```text
["Beloved", "Dune"]
```

The same language can query a knowledge vault, transform JSON from the command line, build an ETL pipeline, traverse a graph, execute queries inside Markdown, or assemble context for an AI agent.

## TL;DR

- **Typed functional query language** — expressions, functions, pattern matching, collections and pipelines, checked before execution.
- **Data contracts** — data read from outside is checked against declared types and rules where it enters, and a violation names each rule it broke.
- **SQL-like queries** — where, order by, joins, group by and aggregates as pipeline steps over typed rows.
- **Dataflow language** — compose transformations over finite data, streams and batches.
- **Data integration and ETL** — connect files, standard streams, databases, queues and other sources and targets through one query model.
- **AI-native language** — retrieval, context construction and agent workflows are expressed as transparent data pipelines.
- **Executable Markdown** — run HQL cells inside HyperMarkDown documents and directly in VS Code, turning Markdown into live queries, reports, views and AI-enabled knowledge artifacts.
- **Knowledge graph language** — retrieve, traverse, expand and combine structured knowledge and relationships.
- **Command-line data tool** — process JSON, YAML, CSV, text and streams in the role normally filled by tools such as `jq`, while retaining types, functions and reusable programs.
- **Streaming by design** — `Stream<T>` is a first-class type; pipelines can process values as they arrive instead of materializing an entire dataset.
- **One language across execution surfaces** — use HQL from the CLI, `.hql` programs, executable documents and interactive editor cells.

Documentation: [hql-lang.org](https://hql-lang.org/)

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

### A general-purpose core

Beyond queries, HQL has the pieces a program needs: records, unions, results,
traits, receiver calls and modules.

```hql
type Circle { radius : Float }
type Rect { width : Float, height : Float }

trait Shape {
    fn area(self) : Float
    fn describe(self) : String = fmt("area {}", self.area())
}

impl Shape for Circle {
    fn area(self) : Float = 3.0f * self.radius * self.radius
}

impl Shape for Rect {
    fn area(self) : Float = self.width * self.height
}

fn total<T : Shape>(shapes: List<T>) : Float = shapes | map(s => s.area()) | sum

[Circle(radius = 1.0f).describe(), str(total([Rect(width = 2.0f, height = 3.0f)]))]

```

A record is constructed by its fields and read with a dot; a `match` over a
union of records covers every member; `Result<T, E>` is `ok(v)` or `err(e)`
and there is no unwrap. `value.f(args)` calls `f` with the value first,
resolved by the value's type among the visible modules, so `"  hi ".trim()`,
`xs.first()` and `7.max(3)` need no import. `trait` and `impl` are HQL's own
capability contracts, verified by the checker: the operators resolve through `Add`, `Eq` and `Ord`,
and a type implementing `Ord` sorts. `import a.b as c` makes one module
visible, from the standard library or from a module root of HQL files;
every module file opens with `module a.b`, its name checked against its path,
and `module a.b for String` makes its `self` functions receiver functions. The
[Mighty Types corpus](examples/mighty-types/README.md) walks all of it,
program by program.

### Checked before it runs

`hql check` and every run check the whole program before a step evaluates. A
generator declared as a `Stream` may not end, so a step that needs the whole
input is refused at check time, not after it has run forever:

```hql
fn naturals(from: Int) : Stream<Int> {
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}
naturals(1) | sum
```

```text
check error[type-mismatch]: `sum` needs a finite input, and Stream<Int> may not end; bound it first with `take(n)`, or batch it with `collect(n)`
 --> query.hql:7:15
  |
7 | naturals(1) | sum
  |               ^^^
```

`naturals(1) | take(5)` is `[1, 2, 3, 4, 5]`. A diagnostic names its phase
(`load`, `parse`, `check`, `plan`, `execute`, `render`), its severity and a
stable kind.

---

## SQL-like queries

Select, where, order by, joins, group by and the aggregates are pipeline
steps over typed rows:

```hql
type Book { title : String, author : String, pages : Int }
type Author { id : String, name : String }
type Row { author : String, pages : Int }

books = [
    Book(title = "Dune", author = "herbert", pages = 412),
    Book(title = "Emma", author = "austen", pages = 474),
    Book(title = "Persuasion", author = "austen", pages = 249),
]
authors = [
    Author(id = "herbert", name = "Frank Herbert"),
    Author(id = "austen", name = "Jane Austen"),
]

books
| join(authors, left = b => b.author, right = a => a.id,
       into = (b, a) => Row(author = a.name, pages = b.pages))
| group(by = r => r.author)
| map(g => {author: g.key, books: g.items | size, pages: g.items | map(r => r.pages) | sum})
```

```text
{{author: "Frank Herbert", books: 1, pages: 412}, {author: "Jane Austen", books: 2, pages: 723}}
```

`left_join`, `right_join`, `full_join`, `cross_join`, `semi_join` and
`anti_join` complete the join family; `HAVING` is a `filter` after `group`.

---

## Data contracts at the boundary

Data read from outside is `Data`, a JSON-shaped tree. Piping it into a type
reads it as that type, and a `DataContract` states what such a value must
hold beyond its shape:

```hql
fn is_country_code(code: String) : Bool = code.length() == 2 and code.upper() == code

type User : DataContract = {
    id: String
    name: String
    age?: Int
    country: String

    require name != ""
    require age? >= 0
    validate country with is_country_code
}

rows : List<Data> = [
    {id: "u-1", name: "Alice", country: "CH"},
    {id: "u-2", name: "", country: "Switzerland"},
]
rows | User
```

```text
execute error[contract-violation]: this does not read as User: `require name != ""` does not hold; `validate country with is_country_code` does not hold
```

`validate(User)` gives the same check as a value, `ok(user)` or an `err`
carrying each `Violation`, for a program that keeps the rejected rows rather
than stopping. A contract also declares defaults, warnings, and rules across
fields such as `require status == "closed" => closed_at?`.

---

## Failures are data

A failure a program expects is a `Result`; one it does not expect is a
diagnostic. `Result` values flow through pipelines like any other:

```hql
fn parse(text: String) : Result<Int, String> = match text.to_int() {
    some(n) => ok(n),
    none => err(fmt("not a number: {}", text)),
}

results = ["4", "x", "15"] | map(parse)
{
    total: results | oks() | sum,
    rejected: results | errs(),
    fallback: parse("x").unwrap_or(0),
}
```

```text
{fallback: 0, rejected: ["not a number: x"], total: 19}
```

Under `--report collect`, `map`, `filter` and `flat_map` step past an element
whose function failed, report it, and keep the rest.

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
| take(1000)
```

bounds a stream that may not end and gives:

```text
List<Event>
```

and `emit`, the inverse of `collect`, passes on every member of every batch,
in order:

```hql
[[1, 2], [3]] | emit
```

```text
[1, 2, 3]
```

Branches are part of the same model. `tee` binds the result of a branch to a
name and passes its input on unchanged:

```hql
events
| tee errors =
    filter(e => e.level == "error")
| map(e => e.message)
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
| take(1000)
| target(file("events.json"), Json)
```

bounds the stream and writes one finite result.

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

> [!WARNING]
> **Design only.** Wiring a pipeline to queues, databases and services is not
> in the current release; reading and writing files and standard streams is.
> The book's [ETL pipelines](https://hql-lang.org/latest/learn/15-etl-pipelines/)
> chapter specifies it.

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

bounded:

```hql
events
| take(1000)
```

batched:

```hql
events
| collect(100)
```

or flattened back from batches:

```hql
events
| collect(100)
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
import semantic

context =
    cards
    | semantic("a creature and its maker")
    | take(10)
    | expand(depth = 1)
```

Retrieval is not a terminal operation.

The result remains ordinary typed data that can continue through filtering, graph traversal, transformation, aggregation and rendering.

```hql
import semantic

cards
| semantic("a creature and its maker")
| take(20)
| filter(hit => hit.score > 0.1f)
| expand(depth = 2)
```

This makes HQL a natural language for AI harnesses: retrieval, context construction, prompts, agents and their outputs can be represented as inspectable programs instead of opaque framework callbacks.

> [!WARNING]
> **Design only.** Retrieval and graph expansion ship today. Agents, prompts
> and runs as values, the `ai` module, are not in the current release; the
> book's [Agents and runs](https://hql-lang.org/latest/learn/20-agents-and-runs/)
> chapter specifies them.

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

A card is named with `@[[name]]`, the same wikilink its documents write, and
its links are read in both directions:

```hql
@[[dune]].title                                        // "Dune"
@[[mrs-dalloway]] | uplinks                            // [mrs-dalloway -> virginia-woolf]
@[[virginia-woolf]] | downlinks | map(e => e.source)   // ["a-room-of-ones-own", "mrs-dalloway", "the-common-reader"]
```

A collection of cards becomes a `Graph`, which formats as text, D2 or Mermaid:

```hql
cards
| filter(c => c.name == "virginia-woolf" or c.metadata.author == "Virginia Woolf")
| Graph
| format(Mermaid)
```

Retrieval can find an initial set of nodes:

```hql
import semantic

cards
| semantic("a creature and its maker")
| take(3)
```

and graph traversal can expand from them:

```hql
import semantic

cards
| semantic("a creature and its maker")
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
import lexical

cards
| lexical("a creature and its maker")
| take(3)
```

Semantic search:

```hql
import semantic

cards
| semantic("a creature and its maker")
| take(3)
```

The result can immediately participate in further computation:

```hql
import semantic

cards
| semantic("a creature and its maker")
| take(20)
| filter(hit => hit.score > 0.1f)
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

## Formats are a category of their own

HQL treats data representations explicitly. A format is a value of `Format`:
it describes an external representation and its framing, and it is neither the
text it encodes nor the values it decodes.

```hql
format(result, Json)
format(result, Yaml)
format(result, Markdown)
```

The formats:

```text
Text      Lines
Json      JsonLines
Yaml      Csv
Markdown  Hmd
Hql       D2      Mermaid
```

`format(x, Json)` applies the encoder for `x`'s type and gives the encoded text
as a `String`; `source(e, Json)` applies the decoder. `Lines` frames text one
record per line and `JsonLines` one JSON document per line, so
`source(file("events.jsonl"), JsonLines)` reads records without a `split("\n")`
in sight. The contracts, `Decoder<F, T>` and `Encoder<T, F>`, are declared in
`lib/std/format/mod.hql` and implemented natively.

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
source(stdin, Json)
| sink(stdout, Json)
```

writes each item of a `Stream<Data>` as its own JSON document, as it arrives.

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

not from the format: `source(file("x.jsonl"), JsonLines)` is a `Seq<Data>`
because a file ends, and `source(stdin, JsonLines)` a `Stream<Data>` because
standard input does not. The format decides the representation; the endpoint
decides whether it ends.

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
import semantic

cards
| semantic("decisions affecting event delivery")
| take(10)
```

expanded through relationships:

```hql
import semantic

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
hql '<program>'            the same as hql eval '<program>'
hql eval <program>         check and evaluate a program given as an argument
hql run <file>             check and evaluate a program from a file; - reads stdin
hql check <file>           parse and type-check without evaluating
hql plan <file>            print each pipeline's steps and the stage each runs in
hql repl                   read, evaluate and print, one program at a time
hql render <file>          run the HQL cells in a document
hql builtins               list the steps a pipeline may use
hql help <name>            print the declarations and doc strings of a name
hql boundary <file> <id>   print a configured source's boundary, field by field
hql config                 show the reporting settings in force and their origin
hql doctor                 check the semantic-search chain and name what is missing
```

Choose the presentation format of the final result with:

```bash
hql run query.hql --to json
hql run query.hql --to json --compact    # one line per document, as jq -c
hql run query.hql --to markdown
```

Name the format of standard input once, so the program need not:

```bash
cat events.jsonl | hql --from json 'source(stdin) | map(e => e.kind)'
```

Choose how failures are reported:

```bash
hql run etl.hql --report collect             # step past failing elements, report them all
hql run etl.hql --diagnostics-format json    # one JSON document per diagnostic on stderr
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

**[hql-lang.org](https://hql-lang.org/)**

[The HQL book](https://hql-lang.org/latest/learn/) is read from the first page
to the last, every program in it runs against a small reading-club vault, and
it is in three parts:

- **The language** — data types, collections, functions, modules, SQL-like
  queries, streams, I/O, the type system, data contracts and errors.
- **Data flows** — what `|` does, when a program is checked, where it runs,
  and ETL pipelines.
- **Knowledge and AI** — a vault of documents, links and graphs, retrieval by
  meaning, queries inside documents, agents and the hyper web.

To follow along:

```bash
git clone https://github.com/ewiger/hql && cd hql
hql --vault examples/tutorial eval 'cards | size'
```

The [examples](examples/) are where HQL's features are designed: each query
file declares its expected type and result, and the test suite runs every one.

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
