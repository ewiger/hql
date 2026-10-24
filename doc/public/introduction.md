# Introduction

HQL is a statically typed language for querying, transforming, connecting and
computing over data, knowledge and AI workflows. A program is an expression:
it names where its data comes from, pipes it through a chain of typed steps,
and returns a typed value. The whole program is type-checked before it runs,
so a question that cannot be answered is refused before any data is read.

This program lists the books in a reading-club vault that nobody has read yet,
by title:

```hql
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
```

```text
["Leaves of Grass", "The Common Reader", "The Hobbit"]
```

`cards` is every document in a folder of Markdown files. `filter` keeps the
ones whose header says `status: todo`, `sort` orders them by title, and `map`
takes each title. Each `|` hands one step's result to the next. Change a header
and the next run gives a different answer: the reading list is a query, not a
list somebody keeps up to date.

The same language reads JSON from a pipe, writes YAML to a file, walks a graph
of links, ranks documents by meaning, runs inside a Markdown document, and
assembles the context an AI agent reads. HQL is hard to reduce to "a query
language for X", and is best met through several views of it.

## A typed functional language

Values, bindings, functions and pattern matching are the core. Functions are
values: declared, passed, returned, generic over their types, partially
applied with a hole, and defined as one-line aliases whose types the checker
writes. `match` is checked for exhaustiveness, and `Option` makes absence a
type.

The collections are `Seq<T>`, `List<T>`, `Set<T>` and `Map<K, V>`, each with
its own semantics, and `Data` holds dynamically structured values such as
JSON. Every step states what it accepts and what it returns. Ask for the `sum`
of a list of cards and you are told `sum` takes numbers, not cards, at the
place you wrote it, before anything runs.

A pipeline is an ordinary expression, not a separate query syntax. Grouping is
a function like filtering:

```hql
cards
| filter(c => c.metadata.category == "book")
| group(by = c => c.metadata.author)
| map(g => {author: g.key, books: g.items | size})
```

```text
{{author: "Virginia Woolf", books: 3}, {author: "Frank Herbert", books: 1}, {author: "Mary Shelley", books: 1}, {author: "Walt Whitman", books: 1}, {author: "J. R. R. Tolkien", books: 1}}
```

## A dataflow language

Computation is data moving through transformations. `Stream<T>` is a
first-class type for values that arrive over time and may never end. `map`,
`filter` and `take` run over a stream one value at a time; `collect(n)` turns
it into a stream of batches, and `emit` turns batches back into values. A step
that needs the whole input — `sort`, `sum`, `group` — is refused over a stream
that may not end, because the type system knows which collections end.

`tee` binds a side branch of a pipeline and passes its input on, so one pass
can answer several questions, or write to several places. Dataflow is not an
execution detail beneath HQL; it is how HQL programs are written.

## Sources, targets and sinks

Data enters through **sources** and leaves through **targets** and **sinks**.
An endpoint — `stdin`, `stdout`, `stderr`, `file(path)` — says where bytes
are, and a format type says how they are documents:

```hql
source(file("sightings.csv"))
| filter(row => row.seen == "morning")
| target(file("mornings.json"))
```

`source` over a file gives a sequence that ends; over standard input, a live
stream. `target` writes one completed value; `sink` writes one document per
value, as each arrives. The difference between a whole value, a stream and a
batch lives in the type — `Seq<T>`, `Stream<T>`, `Stream<Seq<T>>` — not in
separate I/O APIs. Channels carry typed values between parts of a program,
and a logger writes structured events to any endpoint.

Reading and writing are **effects**, and the host decides which it allows. A
terminal may write files; a document being rendered reads only beside itself
and writes nothing outside itself. The program model does not change.

## An integration and ETL language

Because sources, transformations, batches and targets share one expression
model, a pipeline that moves data between systems needs no second workflow
language around it. The query is the dataflow. Configured sources extend the
same model beyond files: a source is bound by its id and used directly, its
connection details and secrets kept in the run's configuration, never in the
program:

```hql
source("warehouse")
| filter(o => o.status == "completed")
| collect(500)
| sink("analytics")
```

What a source holds is fixed by its contract — a finite sequence in a stated
order, a keyed store, a replayable stream of messages, a live stream with no
replay — and each kind comes with the extension that reads it.

## A command-line data tool

Without a vault or a project, `hql` processes JSON, YAML, CSV and text in the
role usually filled by `jq`, keeping types, functions and reusable programs:

```text
$ cat sightings.jsonl | hql 'source(stdin, Json) | filter(s => s.seen == "morning") | map(s => s.species)'
"wren"
"kingfisher"
```

`--to json` pretty-prints each streamed document, `--compact` writes one line
each, and `--to yaml` converts. A one-liner moves into a file and runs with
`hql run`.

## Knowledge: vaults, graphs and retrieval

A **vault** is a folder of Markdown or HyperMarkDown files, each with a header
of metadata and `[[links]]` to the others. Its documents are cards, and its
links a graph that a program walks in either direction, grows outward from a
few cards, and draws as a D2 or Mermaid diagram:

```hql
@[[virginia-woolf]] | downlinks | map(e => e.source)
```

```text
["a-room-of-ones-own", "mrs-dalloway", "the-common-reader"]
```

`lexical` ranks cards by shared words; `semantic` ranks them by meaning against
an index built ahead of time. A ranking is ordinary typed data that continues
through the pipeline, and every hit carries how it was scored — the model, the
index, the metric. A node a traversal added says it was reached, not matched.

## An AI-native language

Most of the complexity of an AI application is around the model call:
retrieving evidence, expanding it, building the prompt, giving the model tools,
continuing the conversation, keeping what came back. These are dataflow
problems, and HQL writes them as dataflow:

```hql
import ai

librarian = @[[librarian]] | Agent

cards
| semantic("books about reading")
| take(10)
| Context("evidence")
| run(librarian)
```

An agent is a card: its body is its instructions and its header its
configuration. A context keeps every item with its provenance, a tool is a pure
HQL function, and a `Run` keeps the whole execution — the prompt as sent, the
turns, every tool call, the children it delegated to, the usage — as a value a
program reads. Responses are recorded, so a program that calls a model
evaluates again offline, byte for byte.

## Executable documents

An HQL block in a document is a **cell**. `hql render` runs the cells, in the
order their dependencies require, and writes each answer in its place; cells
share values by name, and every saved answer records a hash of the code that
produced it. In VS Code the
[HyperMarkDown extension](https://github.com/ewiger/hypermarkdown/tree/main/tools/hmd-vsc-ext)
runs the cells interactively.

A document becomes a live index, a generated report, a dashboard, or a prompt
whose instructions are the prose around the cell. The query stays visible next
to the text it supports, and HQL is the same language on the command line, in
a `.hql` program, in a rendered document and in an editor cell.

## Formats are types

A value is written as `Text`, `Hql`, `Markdown`, `Json`, `Yaml`, `Csv`, `D2`
or `Mermaid` by a step, and the result is a value a later step can still read.
The format describes the encoding; the type describes the shape. A stream is
written piece by piece as it arrives.

## Where to go next

- **Try it in five minutes.** [Install](install.md) the binary — one download,
  or `cargo install hql` — clone the repository for the tutorial vault, and run
  the reading-list program above:

  ```sh
  git clone https://github.com/ewiger/hql && cd hql
  hql --vault examples/tutorial 'cards | filter(c => c.metadata.status == "todo") | map(c => c.title)'
  ```

  `hql repl` keeps a session open, with history and multi-line input, and
  `hql builtins` lists every step with its signature.

- **Learn the language.** [The HQL book](learn/index.md) is fifteen chapters
  read in order, from a first value to an agent whose run is a value. If you
  know `jq`, start at [Data](learn/02-data-types/data.md) and
  [Reading and writing data](learn/13-reading-and-writing.md); if you know SQL,
  at [Asking questions](learn/05-questions.md); if you keep notes in a vault,
  at [A vault of documents](learn/06-a-vault.md); if you build with language
  models, at [Agents and runs](learn/15-agents-and-runs.md).

- **Point it at your own notes.** `hql --vault <folder> 'cards | size'` is the
  first question to ask a folder of Markdown.
  [Queries inside documents](learn/11-documents.md) shows how to put the
  answers back into the notes.

- **Look something up.** The [reference](reference.md) states the grammar, the
  commands and the reporting rules precisely, and the [examples](examples.md)
  are small vaults with runnable queries, each pinned by a test.

- **See what is coming.** The [roadmap](roadmap.md) lists what is next.

- **Build or contribute.** [Build and run](../DEVELOP.md) covers building from
  a clone, the tests and the lints; the source is at
  [github.com/ewiger/hql](https://github.com/ewiger/hql).
