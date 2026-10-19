# Introduction

HQL is a typed functional language for asking questions of data that lives
somewhere else: a folder of documents, a database table, a message log, an
API. A program names a source, pipes its contents through a chain of
functions, and returns a typed value. The whole program is type-checked before
it runs, so a question that cannot be answered is refused before any data is
read.

The first source, and the one the binary implements today, is a **vault**: a
folder of Markdown files, each with a header of metadata and links to the
others. This program lists the books in a reading-club vault that nobody has
read yet, by title:

```hql
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
```

```text
[Leaves of Grass, The Common Reader, The Hobbit]
```

`cards` is every document in the folder. `filter` keeps the ones whose header
says `status: todo`, `sort` orders them by title, and `map` takes each title.
Each `|` hands one step's result to the next. Change a header and the next
run gives a different answer: the reading list is a query, not a list somebody
keeps up to date.

## One language over many sources

Nothing in the core of HQL knows what a document is. The core has values,
types, functions and pipelines; everything that reaches outside the program is
an **extension**, and a folder of Markdown is one of them. The same `filter`,
`sort`, `group` and `map` that read a header above run over a record written by
hand, a JSON-shaped tree, a set, a map, or a stream that never ends. A step
that needs the whole input — `sort`, `sum`, `group` — is refused over a stream
that may not end, because the type system knows which collections end.

The design carries this further. A **data source** is a named endpoint outside
the program — a vault, a Postgres table, a Kafka topic, an HTTP API — bound by
its id and used directly:

```hql
warehouse = source("warehouse")
warehouse | filter(o => o.total > 100) | take(10)
```

There is no verb between a source and its first step. What a source holds is
fixed by its contract — a finite sequence in a stated order, a keyed store that
supports `get`, a replayable stream of messages, a live stream with no replay,
or a catalog of further sources — and each kind of source comes with the
extension that reads it. Connection details and secrets live in the run's configuration,
never in the program. How much of a source is read follows from the pipeline,
`take(10)` and `filter` included, and how the host fetches it is not the
language's concern. A relational source yields rows, not documents pretending
to be rows; the vault's documents are cards because the vault extension says
so. The vault is the source `vault`, and `cards` is `vault.cards`.

Today the vault is the only source, and the extensions compiled into the
binary are `core`, `graph`, `format`, `lexical` and `semantic`. Sources beyond
the vault are the direction of the work, not yet something you can run.

## What the binary does today

**Types first.** Every program is checked as a whole. Ask for the `sum` of a
list of cards and you are told `sum` takes numbers, not cards, at the place you
wrote it, before anything runs.

**Pipelines of functions.** No clauses: grouping is a function like filtering.

```hql
cards
| filter(c => c.metadata.category == "book")
| group(by = c => c.metadata.author)
| map(g => {author: g.key, books: g.items | size})
```

```text
[{author: "Virginia Woolf", books: 3}, {author: "Frank Herbert", books: 1}, {author: "Mary Shelley", books: 1}, {author: "Walt Whitman", books: 1}, {author: "J. R. R. Tolkien", books: 1}]
```

Functions are values: declared, passed, returned, generic over their types,
and defined as one-line aliases whose types the checker writes.

**Links are a graph.** A `[[virginia-woolf]]` in a card is a link, and the
links of a vault form a graph that a program walks in either direction, grows
outward from a few cards, and draws as a D2 or Mermaid diagram:

```hql
@[[virginia-woolf]] | downlinks | map(e => e.source)
```

```text
[a-room-of-ones-own, mrs-dalloway, the-common-reader]
```

**Retrieval is evidence.** `lexical` ranks cards by shared words; `semantic`
ranks them by meaning against an index built ahead of time. Every hit carries
how it was scored — the model, the index, the metric — and a node a traversal
added says it was reached, not matched.

**Formats are types.** A value is written as `Text`, `Markdown`, `Json`,
`Yaml`, `Csv`, `D2` or `Mermaid` by a step, and the result is a value a later
step can still read. A stream is written piece by piece as it arrives.

**Documents compute themselves.** An HQL block in a document is a cell.
`hql render` runs the cells, in the order their dependencies require, and
writes each answer in its place; cells share values by name, and every saved
answer records the code that produced it.

## Where to go next

- **Try it in five minutes.** [Install](install.md) the binary — one download,
  or `cargo install hql` — clone the repository for the tutorial vault, and run
  the reading-list program above:

  ```sh
  git clone https://github.com/ewiger/hql && cd hql
  hql --vault examples/tutorial eval 'cards | filter(c => c.metadata.status == "todo") | map(c => c.title)'
  ```

  `hql repl` keeps a session open, with history and multi-line input, and
  `hql builtins` lists every step with its signature.

- **Learn the language.** [The HQL book](learn/index.md) is twelve chapters
  read in order, from a first value to a document that builds itself. If you
  know `jq`, start at [Data](learn/02-data.md); if you know SQL, at
  [Asking questions](learn/05-questions.md); if you keep notes in a vault, at
  [A vault of documents](learn/06-a-vault.md). Every program in it runs as
  printed.

- **Point it at your own notes.** A vault is any folder of Markdown or
  HyperMarkDown files with a header of metadata and `[[links]]` between them;
  `hql --vault <folder> eval 'cards | size'` is the first question to ask it.
  [Queries inside documents](learn/11-documents.md) shows how to put the
  answers back into the notes.

- **Look something up.** The [reference](reference.md) states the grammar, the
  commands and the reporting rules precisely, and the [examples](examples.md)
  are small vaults with runnable queries, each pinned by a test.

- **See what is coming.** The [roadmap](roadmap.md) lists what is next:
  record values and joins, sources beyond the vault, execution backends.

- **Build or contribute.** [Build and run](../DEVELOP.md) covers building from
  a clone, the tests and the lints; the source is at
  [github.com/ewiger/hql](https://github.com/ewiger/hql).
