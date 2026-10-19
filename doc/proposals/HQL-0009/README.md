# HQL-0009: Cells: identity, shared values and saved outputs

**Status**: implemented
**Created**: 2026-10-18
**Source**: [program values](../../models/behavior/program-values.md), its
section "HMD cell contexts"; issue
[0020](../../issues/0020-render-as-compilation.md)

## Abstract

A HyperMarkDown document is a notebook whose cells are `hql` fences. A cell
MAY carry an `id`, which names its value for the other cells of the document.
The cells of a document form one dependency graph: a cell that names another
cell's `id` reads that cell's value, and the document evaluates in dependency
order, never in the order someone last ran it. A cell's bindings stay its own.
Each saved output records which cell produced it and a hash of the source it
came from, so a host can tell a current output from a stale one without running
anything. `hql render` gains a per-cell JSON record and a way to run one cell
and what it depends on, which is the interface an editor extension drives.

## Motivation

- Cells are independent programs today. A value computed in one cell cannot be
  used in the next, so a notebook that builds an answer in steps has to repeat
  every earlier step.
- Jupyter shares state through a kernel's mutable namespace. What a cell sees
  depends on which cells someone ran and in what order, which is invisible in
  the saved file. A document that is a query must mean the same thing however
  it was last run.
- A saved output is tied to its cell only by being the next line after it.
  Nothing says which cell an output belongs to or whether the cell has changed
  since.
- An editor extension needs to run one cell, find its output, and show whether
  it is current. Each of these needs a stable name for the cell and a
  machine-readable result.

## Non-goals

- **No references across documents.** A cell reads cells of its own document.
  `[[card#cell]]` is a later decision.
- **No effects.** `hql#declare` and every other cell that changes a store stay
  out of scope, as the [knowledge model](../../models/domain/knowledge.md)
  records.
- **No kernel.** No process keeps state between runs. A host MAY cache values
  under the rules in the section "Hashes and staleness"; the cache never changes
  a result.
- **No output of rich media.** An output is Markdown, as
  [HQL-0008](../HQL-0008/README.md) requires of a document host.

## Specification

### 1. The info string

A fence is a cell when the first word of its info string is `hql`, `hql#eval`
or `hql#query`. The words after it are attributes.

```text
info      := kind attribute*
kind      := "hql" | "hql#eval" | "hql#query"
attribute := "id=" ident | "replace" | "after" | "hidden"
```

- `id=<ident>` MUST be an HQL identifier, in `snake_case`, and MUST be unique
  in the document. A duplicate is a check error naming both cells. The id is a
  binding name, so it follows the same rule as any other.
- `replace` and `after` set where the output goes, as issue 0020 specifies.
- `hidden` runs the cell and writes no output. It is for a cell that exists to
  be read by other cells.
- An unknown attribute MUST be a warning naming it, and is otherwise ignored.
  The info string is shared with Markdown renderers, which ignore every word
  after the first.

````markdown
```hql#eval id=todo
cards | filter(c => c.metadata.status == "todo")
```
````

### 2. Cell scope

A cell is a closure over the card environment, as
[program values](../../models/behavior/program-values.md) records: `this:
Card`, the header's imports, and the card-level bindings. This record adds the
cell values.

- Every cell MUST see, as bindings, the `id` of every other cell in the
  document, each bound to that cell's value.
- A binding made inside a cell MUST NOT be visible to any other cell. The only
  thing a cell exports is its value, under its `id`, so what flows between
  cells is written in the document.
- A cell's own binding MAY shadow a cell id, and then refers to the binding
  inside that cell only.
- A cell without an `id` exports nothing. It still sees every other cell.

### 3. The value of a cell

- A cell's value is the value of its program: its last expression. A cell
  that ends in a format, `| markdown`, exports that text, which is an ordinary
  value ([HQL-0017](../HQL-0017/README.md)).
- A cell's checked type is the type of its value, and that is the type its id
  has in other cells.

````markdown
```hql#eval id=todo
cards | filter(c => c.metadata.status == "todo")
```

```hql#eval
size(todo)
```
````

The first cell's output is the list of open cards, and `todo` is the
`Seq<Card>` it shows.

### 4. Evaluation order

The cells of a document form a graph: an edge runs from a cell to every cell
whose id it reads as a free name.

- The graph MUST be acyclic. A cycle is a check error naming every cell on it.
- A host MUST check and evaluate cells in an order where every cell comes after
  the cells it reads. Among cells that do not depend on each other, document
  order breaks the tie, so a run is deterministic.
- A document is checked as a whole before any cell is evaluated, so a type
  error in a late cell stops a build before an early one runs.
- When a cell fails, every cell that depends on it, directly or through other
  cells, is **skipped**: it does not run, and its output says which cell it
  waited on. Cells that do not depend on the failure still run.

### 5. Saved outputs

An output under `after` sits between two markers, as issue 0020 specifies. The
opening marker carries what the output was produced from.

```text
<!-- hql:output id=<ident> hash=<12 hex> status=<status> -->
...Markdown...
<!-- /hql:output -->

status := "ok" | "error" | "skipped"
```

- `id` MUST be present when the cell has one, and absent otherwise.
- `hash` MUST be the cell's hash, defined in the next section, when the output
  was written.
- `status` is `ok` for a value, `error` for a failed cell, whose output is its
  reports, and `skipped` for a cell that waited on a failure, whose output is
  `skipped: <id> failed`.
- An output MUST follow its cell, separated from the closing fence by nothing
  but blank lines. A host MUST attach it to that cell.
- An output with an `id` whose cell is not directly above it is a warning
  naming the id, and the next render removes it. An output with no cell above
  it is removed the same way. A document never accumulates outputs nobody owns.

### 6. Hashes and staleness

A cell's hash covers its source and the hashes of the cells it reads, so that
editing a cell makes every output downstream of it stale.

```text
hash(cell) = hex(sha256(
    "hql-cell\0" || normalized(source) || "\0" ||
    join("\0", sorted(dep.id || "=" || hash(dep) for dep in reads(cell)))
))[0..12]

normalized(source) = source with trailing whitespace stripped from each line,
                     and trailing blank lines removed
```

- An output is **current** when its `hash` equals the cell's hash computed from
  the document as it now stands, and **stale** otherwise. A host computes this
  without running anything.
- The hash covers code only. A change to the vault does not change any hash,
  so a current output means "produced by this code", not "true of the vault
  today". `hql render` always evaluates; it never skips a cell because its hash
  matches.
- A host MAY reuse a value it computed earlier for a cell whose hash and vault
  are unchanged. Reuse MUST NOT change any output.

### 7. The command

`hql render` keeps the options issue 0020 specifies and gains two.

- `--cell <id>` MUST evaluate that cell and every cell it depends on, and
  rewrite only that cell's output. The other outputs are left as written. It
  is how an editor runs one cell. Under `--emit replace` it is a usage error,
  exit `2`, since a build is of the whole document.
- `--envelope json` MUST print one JSON object instead of the document:

```text
{
  "document": "<the rendered document>",
  "cells": [
    {
      "id": "todo" | null,
      "index": 0,
      "span": [start, end],
      "hash": "3f2a9c1e04b7",
      "type": "List<Card>",
      "status": "ok" | "error" | "skipped",
      "markdown": "...",
      "reports": [ ... ]
    }
  ]
}
```

- `index` is the cell's position in document order, from `0`. `span` is the
  zero-based UTF-8 byte range of the cell's fences in the input, closing fence
  included, as every other range HQL reports. `reports` has the shape the
  report queue already has in `--envelope json`. The object also carries the
  document-level `reports`.
- The `cells` array MUST be in document order, whatever order evaluation took.

## Backwards Compatibility

A document from issue 0020 renders unchanged: its cells have no ids and read
no other cell, and its output markers carry no attributes. The next render
under `after` adds `hash` and `status` to each opening marker. A cell whose
program happened to use a name that a later cell now takes as its id sees that
cell's value only if the name was free in it; a binding of its own still wins,
so no existing cell changes meaning.

## Security Considerations

A cell is pure, so reading another cell's value adds no effect. A cycle is
rejected at check time and a skipped cell does not run, so a document cannot
make evaluation loop through its cells. A document with many cells is bounded
by the same limits as one program of the same total size.

## Reference Implementation

- `src/transclude.rs`: the info-string attributes, attaching outputs across
  blank lines, the marker attributes, removing ownerless outputs, `--cell`, the
  cell graph, cycle detection, dependency order, the hash, and the scope that
  binds each id
- `src/checker` and `src/execution`: checking and evaluating a cell in an
  environment extended with typed cell values
- `src/cli.rs`: `--cell`, and the JSON record under `--envelope json`
- `doc/models/behavior/program-values.md`: the section "HMD cell contexts"
  states the scope and order rules and drops them from what it lists as open

## Test Plan

Unit tests MUST include:

- a cell reads an earlier cell's value by id, and a later one's
- a binding inside a cell is not visible to another cell
- a cell ending in `| markdown` exports that text, at type `Markdown`
- two cells that read each other are one check error naming both
- a duplicate id is a check error naming both cells
- a failed cell skips its dependents and not the cells beside them
- the hash changes when a cell's source changes, and when a cell it reads
  changes, and not when an unrelated cell changes or a line gains trailing
  spaces

Integration tests MUST include:

- `--emit after --in-place` on a notebook of three dependent cells, run twice,
  is byte-identical
- an output separated from its cell by a blank line is replaced, not
  duplicated
- an output with no cell above it is removed with a warning
- `--cell` rewrites one output and leaves the others byte-identical
- `--envelope json` lists every cell in document order with its id, hash, type
  and status
- `--cell todo --emit replace` exits `2`

```bash
cargo test --test transclude --test cli
```

## Changelog

- 2026-10-18: drafted
- 2026-10-19: implemented: ids, cell scope over the cell values, dependency
  order, skipping, saved outputs with hashes, `--cell` and the JSON record.
  The card environment a cell closes over, `this: Card` and card-level
  bindings, is the [program values](../../models/behavior/program-values.md)
  model's and is not built.
- 2026-10-19: the reference implementation and test plan name
  `src/transclude.rs` and `--envelope json`, where the cell graph and the JSON
  record are built
