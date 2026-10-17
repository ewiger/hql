# The command-line host

Status: implemented.

`hql` is one host among several. It supplies a vault from outside, chooses a
reporting mode, and decides what to do with the value a program returns — which
is the execution context [program values](program-values.md) describes, not a
privilege the language grants it.

Everything but argument dispatch lives in the library, because logic only the
binary can reach is logic that cannot be tested.

## Commands

| Command | What it does |
| --- | --- |
| `hql eval <program>` | check and evaluate an argument |
| `hql check <file>` | print the type without evaluating |
| `hql run <file>` | check and evaluate a file |
| `hql repl` | read, evaluate and print, keeping bindings |
| `hql render <file>` | run the HQL cells in a document and build the Markdown document they yield |
| `hql builtins` | list the steps a pipeline may use, grouped by provider |
| `hql config` | the reporting mode in force, and where it came from |

`-` reads standard input wherever a file is taken, so `hql` composes in a
pipeline.

## Options

| Option | Effect |
| --- | --- |
| `--vault <DIR>` | the namespace `cards`, traversal and retrieval run against |
| `--format text\|json` | a value for a person, or a document for a program |
| `--report strict\|collect` | when the run stops; see [reporting](reporting.md) |

Without `--vault`, `cards`, `downlinks` and `expand` say they need one rather
than quietly finding nothing. The language requires one too: card-ness is a
place in a namespace, and a vault is a directory with `.hmd/` in it — the config
file inside is optional and defaults for it are assumed, the directory is not.
See [Card](../../wiki/hql/types/card-type.hmd) and
[the semantic-search requirements](../requirements/semantic-search.md).

## Extensions

`hql builtins` groups steps by the extension that provides them and says, of
each extension, whether this vault has it: the core is always available, the
prelude arrives unless the vault declines it, and everything else is written
out where it is used. Every *registered* extension is listed, not only what the
vault resolves — otherwise a reader told to write `import semantic` would have
nowhere to look it up.

A vault configures its own imports in `hql.toml`, beside
[the reporting mode](reporting.md):

```toml
[extensions]
import  = ["semantic"]   # imported for every program this vault runs
prelude = false          # optional; omits graph and present
```

`import` here saves writing the same line at the head of every query in one
vault; a query that travels between vaults carries its own `import` statement
instead — see [the grammar](expressions.md). A configured import that names no
registered extension, or that collides with something already imported, fails
every program in the vault rather than failing quietly, and the diagnostic says
it came from `hql.toml`.

## Exit status

`0` succeeded, possibly with warnings. `1` is a language failure or an I/O one.
`2` is a usage error, which clap reports.

## Output

In `text`, the value goes to stdout and the report queue to stderr, so a
pipeline downstream reads the answer and a person reads the complaints. More
than one report is followed by a count of each severity.

In `json`, one object carries everything:

```json
{ "origin": "<expression>", "type": "Int", "value": 42, "reports": [] }
```

## The REPL

Bindings persist by replaying the statements that succeeded, which keeps a
session a program rather than a sequence of unrelated runs. `:type <expression>`
answers without evaluating, `:vault` says what the session runs against,
`:reset` forgets the bindings, and `:quit` leaves.

## Documents that carry queries

`hql render` is the second host. A block fenced `hql`, `hql#eval` or `hql#query`
is a cell. Rendering runs each cell against the vault and writes its output as
Markdown, so the source document compiles to a built one.

```
hql render <FILE> [--write <OUT> | --in-place] [--emit replace|after]
```

- `--emit replace`, the default, writes each cell's output in the cell's place.
  The built document has no cells left in it.
- `--emit after` keeps each cell and writes its output immediately after it,
  between `<!-- hql:output -->` and `<!-- /hql:output -->`. Markdown previews do
  not display the markers. The next render replaces what is between them, so
  rendering twice leaves what rendering once did.
- A cell's info string may name its own mode: `` ```hql#eval after ``.
- Without `--write` or `--in-place` the built document goes to standard output.
  `--write <OUT>` writes it to `OUT`; `--in-place` writes it over `FILE` and is
  a usage error, exit `2`, unless `--emit after` is given, since replacing the
  cells would remove them from the source.

````markdown
```hql#eval
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
```
<!-- hql:output -->
- Bearer tokens
- Session cookies
<!-- /hql:output -->
````

The output is Markdown, as [HQL-0008](../../proposals/HQL-0008/README.md)
requires of a document host. A cell without a presentation is presented with
`markdown`: a record becomes a pipe table, a sequence a bulleted list, a card a
wikilink in that list. `table` becomes a pipe table, `json` a fenced `json`
block, and `text` and `markdown` are written as they are.

A cell that fails under `replace` is left as written and the build is not
written anywhere; the exit code is `1`. Under `after` the error is written
between the markers, as a notebook cell shows one, and the exit code is still
`1`.

This is the console implementation of the direction
[program values](program-values.md) records for `hql#eval`, `hql#query` and
`hql#declare`. A host that draws rather than prints — an editor extension
showing the result inline — consumes the same value; only the presentation
differs. `hql#declare` is not implemented, because contributing knowledge to a
store is the effect boundary the [knowledge model](../domain/knowledge.md)
records as undrawn.

Tests: `tests/cli.rs`, `tests/transclude.rs`.
