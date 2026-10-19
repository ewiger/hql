# 11. Queries inside documents

A document can hold the queries that compute parts of it. `hql render` runs
them and builds the document they describe, so a reading list, an index or a
status table is a query, not a list somebody keeps up to date.

## Cells

A fenced block whose info string is `hql` — or `hql#eval`, or `hql#query` — is a
**cell**. This is [`examples/notebook/reading-list.hmd`](https://github.com/ewiger/hql/tree/main/examples/notebook/reading-list.hmd):

````markdown
# The club's reading list

Built from the vault with `hql --vault examples/tutorial render`.

```hql id=books hidden
cards | filter(c => c.metadata.category == "book")
```

## Still to read

```hql id=todo
books
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
```

## How far along

```hql
{books: books | size, to_read: todo | size}
```
````

## Building the document

`hql render` runs every cell against the vault and writes the document with
each cell replaced by its output, as Markdown:

```sh
hql --vault examples/tutorial render examples/notebook/reading-list.hmd
```

````markdown
# The club's reading list

Built from the vault with `hql --vault examples/tutorial render`.

## Still to read

- Leaves of Grass
- The Common Reader
- The Hobbit

## How far along

| key     | value |
| ------- | ----- |
| books   | 7     |
| to_read | 3     |
````

The source is the program; the built document is its result. `--write
<file>` writes the build to a file instead of standard output.

## Cells share values

A cell's info string carries attributes after its kind:

- `id=<name>` exports the cell's value to the other cells of the document,
  under that name. `books` and `todo` above are such names.
- `hidden` runs the cell and writes nothing, for a value other cells read.
- `replace` or `after` chooses where that one cell's output goes.

Only a cell's value leaves it: a binding inside a cell stays in that cell. The
cells form a dependency graph, and run in the order their reads require, not
the order they are written in. A cycle or a duplicate id is an error naming the
cells, and a cell that reads a failed one is skipped, saying which cell it
waited on.

The document is checked as a whole before any cell runs, so a type error in
the last cell stops the build before the first one runs.

## Keeping the cells

`--emit after` keeps every cell and writes its output after it, between two
comments a Markdown preview does not display:

````markdown
```hql id=todo
books
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
```
<!-- hql:output id=todo hash=9c9f41e99b94 status=ok -->
- Leaves of Grass
- The Common Reader
- The Hobbit
<!-- /hql:output -->
````

With `--in-place` that rewrites the source itself, the way a notebook saves its
outputs, and the next render replaces what is between the markers. The opening
marker records the cell's id, a **hash** of its code and of the code of every
cell it reads, and its status. An output whose hash differs from the cell's
current one is stale, and an editor can say so without running anything.

`--cell <id>` runs one cell and the cells it reads, and rewrites only its
output. `--envelope json` prints the built document and a record per cell —
its id, span, hash, type, status, output and reports — for an editor to read.

## What is next

[Chapter 12](12-working-with-hql.md) is about the tools around the language:
the REPL, the reports, and what `hql` says when something is wrong.
