# 18. Queries inside documents

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

## Cells that maintain a vault

!!! warning "Design only"

    This section describes functionality that is not in the current release.
    It is written as if it existed: these cells do not run against the
    current binary, and names, types and outputs may change before they ship.

A vault's upkeep is a set of questions asked of the vault itself: which cards
are related to this one, what does each category hold, which card does
nothing link to. Each is a cell a card carries, and the hash in its saved
output says when the answer is stale, so maintenance is a render, not a
chore. A card of a vault's own wiki might end with these:

````markdown
## Related

```hql#eval id=related
cards | semantic(this.title) | take(3) | map(h => h.value)
```

## Index

```hql#eval id=index
cards
| group(by = c => c.metadata.category)
| map(g => {category: g.key, cards: g.items | map(c => c.name)})
```

## Health

```hql#eval id=orphans
cards | filter(c => (c | downlinks | size) == 0) | map(c => c.name)
```
````

`this` is the card the cell is written in, so the related cards are the
three nearest in meaning to its title, written below its body as links. A
cell marked `cached` would keep its output until the hash of its code and
its inputs changes, so a summary a model wrote ([chapter 19](19-agents-and-runs.md))
is computed once and read many times. The cell kinds exist today; `this`,
`cached` and a `semantic` over a question the index was not built for are
design direction.

## What is next

[Chapter 19](19-agents-and-runs.md) gives a language model the cards a query
selected, runs an agent over them, and keeps the run as a value.
