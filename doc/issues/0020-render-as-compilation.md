# 0020 — `hql render` compiles a document

Status: done
Decisions: [HQL-0008](../proposals/HQL-0008/README.md), its section
"Presenting into a document"
Related: [program values](../models/behavior/program-values.md), its section
"HMD cell contexts"; [CLI](../models/behavior/cli.md), its section "Documents
that carry queries"

`hql render` treats an `hql#eval` block as a notebook cell: it runs the block
and writes the answer beneath it in an `hql#result` fence. The fence is
console-shaped. Its body is the space-aligned terminal table, so the answer is
not Markdown and the document never reads as if it were written by hand.

Rendering becomes compilation. The `.hmd` file is the source; what `render`
writes is the built document, in which each cell's output is plain Markdown.

## The command

```
hql render <FILE> [--write <OUT> | --in-place] [--emit replace|after]
```

- Without `--write` or `--in-place`, the built document goes to standard
  output and nothing is written.
- `--write <OUT>` writes the built document to `OUT`. `--write` stops being a
  flag that overwrites the input.
- `--in-place` writes the built document over `FILE`. It is accepted only with
  `--emit after`: `replace` removes the cells, so writing it over the source
  would destroy the queries. `--in-place --emit replace` is a usage error, exit
  `2`.
- `--emit replace` removes each cell and writes its output where the cell was.
  It is the default.
- `--emit after` keeps each cell and writes its output immediately after it,
  between two HTML comments:

  ````markdown
  ```hql#eval
  cards
  | filter(c => c.metadata.status == "todo")
  | map(c => c.title)
  ```
  <!-- hql:output -->
  - Ring the barn owl box
  - Count the ravens
  <!-- /hql:output -->
  ````

  Markdown previews do not display HTML comments, so only the output shows. The
  next render replaces everything between the markers, so rendering twice
  leaves what rendering once did. An editor extension reads the same markers to
  draw a cell's output inline.

A block's info string may override the mode for that block:
`` ```hql#eval replace `` or `` ```hql#eval after ``. The fences `hql`,
`hql#eval` and `hql#query` are all cells.

## Output

The output of a cell is its value written in the `hmd` presentation format
([HQL-0017](../proposals/HQL-0017/README.md)): Markdown text is spliced in, and
text of another format is a fenced block with that format's info string.

## Failure

- Under `replace`, a cell that fails fails the build: every report is printed,
  nothing is written, and the exit code is `1`.
- Under `after`, the failure is written between the markers, the way a notebook
  cell shows its error. The reports are still printed, and the exit code is
  `1`.

## Work

1. **Markdown presentation.** The `markdown` presentation and the pipe-table
   form of `table` from HQL-0008, in `src/extensions/present.rs` and
   `src/types/display.rs`.
2. **The renderer.** `src/transclude.rs`: the two emit modes, the
   `<!-- hql:output -->` markers, the per-block override, and the removal of
   `hql#result` and its `RESULT` constant. A leftover `hql#result` block in an
   input is left as written.
3. **The command.** `src/cli.rs`: `--write <OUT>`, `--in-place`, `--emit`, and
   the usage error for `--in-place --emit replace`.
4. **Tests.** `tests/transclude.rs` and `tests/cli.rs`: both modes, the
   markers being idempotent, the override, failure under each mode, and the
   in-place refusal. `tests/fixtures/board.hmd` moves to the new form.
5. **Documentation.** `README.md` (its section "Queries inside documents" and
   the commands table), `doc/models/behavior/cli.md`,
   `doc/public/documents.md`, `doc/public/reference.md`, the module comment in
   `src/transclude.rs`, and the birds and mapreduce example cards that show
   `hql#result`.

## Done when

- No source file, test, example or page mentions `hql#result`.
- `hql render card.hmd --write out.hmd` writes a document with no HQL fences
  left in it, and its tables are Markdown pipe tables.
- `hql render card.hmd --in-place --emit after` run twice leaves the file
  byte-identical to running it once.
- `hql render card.hmd --in-place` exits `2` and does not touch the file.
