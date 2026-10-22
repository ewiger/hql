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
| `hql plan <file>` | check a file and print its plan on the backend in force without evaluating: per pipeline a statement is, the line and the expression its first step reads, then each step with its operation, shape, purity, stage and strategy; `--envelope json` writes the same as one JSON document |
| `hql repl` | read, evaluate and print, keeping bindings |
| `hql render <file>` | run the HQL cells in a document and build the Markdown document they yield |
| `hql builtins` | list the steps a pipeline may use, grouped by provider, each with its signature, purity and shape, then the combiners a fold names with the laws each holds |
| `hql config` | the reporting mode, the shown level and the execution backend in force, and where each came from |
| `hql doctor` | check the semantic-search chain in order — vault, `hql.toml`, index, model, query vectors, card coverage — printing one line per part; it stops at the first missing part, names the command to run next, and exits `1` when any part is missing |

`-` reads standard input wherever a file is taken, so `hql` composes in a
pipeline. A first argument that is not a command name is a program:
`hql '<program>'` is `hql eval '<program>'`. The command line names no input
and no output of a program's own: a program reads with `source` or `tap` and
writes with `target` or `sink` ([HQL-0021](../../proposals/HQL-0021/README.md)),
and a result of type `Unit` is written as nothing.

## Options

| Option | Effect |
| --- | --- |
| `--vault <DIR>` | the namespace `cards`, traversal and retrieval run against; without it, the nearest directory at or above the current one that holds `.hmd/` or `hql.toml` |
| `--envelope text\|json` | what the answer is wrapped in: the value alone on stdout, written in the `--to` format, or one JSON document with the type, the value and the reports |
| `--to <format>` | the [presentation format](../../wiki/hql/presentation.hmd) a result is written in when the program did not format it: `hql` (the default), `text`, `markdown`, `hmd`, `json`, `yaml`, `csv`, `d2`, `mermaid`; `none` evaluates and writes no value |
| `--compact` | a `Json` result written one document per line, as `jq -c` does; without it each document is pretty-printed, as `jq .` does |
| `--log-level trace\|debug\|info\|warn\|error` | every logger's threshold, overriding the one it was constructed with |
| `--report-level debug\|info\|warning\|error` | the lowest severity shown in `text`; see [reporting](reporting.md) |
| `--report strict\|collect` | when the run stops; see [reporting](reporting.md) |
| `--backend <name>` | the execution backend that runs a checked program; see *Backends* below |

The vault is found the way `hmd` finds a project: walking up from the current
directory, the first directory holding `.hmd/` — or `hql.toml`, for a vault
only HQL configures — is the root. `--vault` overrides the walk. Outside every
vault there is no vault, and a program that does not name one still runs, so
`hql eval '1 + 1'` needs none. `cards`, `downlinks` and `expand` say they need
one rather than quietly finding nothing. The language requires one too:
card-ness is a place in a namespace, and a vault is a directory marked as one by
`.hmd/` or `hql.toml`. The config file inside `.hmd/` is optional and defaults
for it are assumed.
See [Card](../../wiki/hql/types/card-type.hmd) and
[the semantic-search requirements](../requirements/semantic-search.md).

## Backends

A checked program runs on an execution backend
([HQL-0004](../../proposals/HQL-0004/README.md)). The backend changes how a
program runs and never its answer, so it is host configuration and never
syntax, resolved with the precedence [reporting](reporting.md) uses:

1. the query — `--backend <name>`
2. the vault — `hql.toml`, `[execution] backend = "<name>"`
3. the user — `HQL_BACKEND` in the environment
4. the default — `localhost`

```toml
[execution]
backend = "localhost"
```

`localhost` is the only registered backend: the in-memory evaluator, each step
run to completion before the next. Before a backend runs a program, the planner
annotates every step with its stage
([HQL-0005](../../proposals/HQL-0005/README.md)); `localhost`'s plan is one
stage per pipeline, in process, and `hql plan` prints it. A name that is not registered fails every
program, and `hql config`, with a message naming the flag, the file or the
variable that supplied it. `hql render` runs its cells on `localhost`.

## Data sources

A vault declares the data sources its programs read under `[sources.<id>]`
in `hql.toml` ([HQL-0012](../../proposals/HQL-0012/README.md)). A program
names a source by its id, `source("field_notes")`, and nothing else: paths,
hosts and credentials are configuration. Paths resolve against the vault
root.

```toml
[sources.field_notes]
kind = "vault"          # a Vault: documents read as cards
root = "notes"

[sources.counts]
kind = "csv"            # also json (one array's elements), json_lines, yaml
path = "data/counts.csv"
element = "Tally"       # optional: the element type; Data by default

[sources.birds]
kind = "sqlite"         # one table, a Data row per element
path = "data/birds.sqlite"
table = "birds"
order = ["id"]          # optional: the row order; rowid by default

[execution]
max_elements = 100000   # the most elements of one source an evaluation holds
max_bytes = 50000000    # the most bytes of one source an evaluation reads
```

Loading the vault refuses a malformed entry, and so fails every program in
it: an id off `[a-z][a-z0-9_]*`, the id `vault` (the vault the program runs
against), an unknown kind, a missing or unknown key, and a literal secret in
`password`, `token`, `secret` or `api_key`. A secret is given as the name of
the environment variable holding it, in a key ending `_env`. Exceeding a
bound is an evaluation error naming the source and the bound; a source is
never truncated.

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
prelude = false          # optional; omits graph, present and format
```

`import` here saves writing the same line at the head of every query in one
vault; a query that travels between vaults carries its own `import` statement
instead — see [the grammar](expressions.md). A configured import that names no
registered extension, or that collides with something already imported, fails
every program in the vault rather than failing quietly, and the diagnostic says
it came from `hql.toml`.

It also chooses, per type and [presentation format](../../wiki/hql/presentation.hmd),
the renderer that writes it, over built-in defaults:

```toml
[format.Graph]
markdown = "mermaid"     # a Graph written as Markdown is a Mermaid block
```

A renderer or format this binary does not have, or a renderer named for a
format or type it does not write, fails loading the vault, so the mistake is
reported before any query reaches it.

## Streams

A result that is a stream that may not end is written piece by piece, each
flushed as it arrives, in the presentation format the program or `--to`
chose. `hql eval 'naturals(1) | json' | head -3` prints three lines and exits
`0`: the reader stopping ends the stream. `--envelope json` refuses such a
result, because a JSON envelope is one document.

## Exit status

`0` succeeded, possibly with warnings. `1` is a language failure or an I/O one.
`2` is a usage error, which clap reports.

## Output

In `text`, the value goes to stdout and the report queue to stderr, so a
pipeline downstream reads the answer and a person reads the complaints. Only
entries at or above the shown level are written, `warning` by default, so a
`debug` capture is silent until `--report-level debug` asks for it. More than
one report is followed by a count of each severity.

In `json`, the queue holds every entry whatever the shown level.

One object carries everything:

```json
{ "origin": "<expression>", "type": "Int", "value": 42, "reports": [] }
```

## The REPL

Bindings persist by replaying the statements that succeeded, which keeps a
session a program rather than a sequence of unrelated runs. `:type <expression>`
answers without evaluating, `:vault` says what the session runs against,
`:reset` forgets the bindings, and `:quit` leaves.

An input continues on the next line while one of its brackets — `(`, `[` or
`{` — is open, and is evaluated whole once they close; a string literal cannot
span lines, so brackets alone decide. A line opening with `|` continues the
input before it, so a pipeline written one step per line runs as one input:
piped input reads ahead for it, and a line typed at the prompt is joined to the
input before it and run again with it. A session keeps every input that
declares — a binding, `fn`, alias, `type` or `import` — and no expression. On a
terminal the line editor recalls earlier inputs, a multi-line one whole, from a
history kept in `~/.hql_history`; Ctrl-C abandons the input being typed and
Ctrl-D on an empty line leaves.

## Documents that carry queries

`hql render` is the second host. A block fenced `hql`, `hql#eval` or `hql#query`
is a cell. Rendering runs each cell against the vault and writes its output as
Markdown, so the source document compiles to a built one.

```
hql render <FILE> [--write <OUT> | --in-place] [--emit replace|after] [--cell <ID>]
```

- `--emit replace`, the default, writes each cell's output in the cell's place.
  The built document has no cells left in it.
- `--emit after` keeps each cell and writes its output immediately after it,
  between an opening marker, `<!-- hql:output id=… hash=… status=… -->`, and
  `<!-- /hql:output -->`. Markdown previews do not display the markers. The
  next render replaces what is between them, so rendering twice leaves what
  rendering once did.
- A cell's info string carries its attributes: `id=<name>`, which exports its
  value to the document's other cells; `replace` or `after`, its own mode; and
  `hidden`, which runs it and writes no output.
- `--cell <ID>` runs that cell and the cells it reads, and rewrites only its
  output. It needs `--emit after`. `--envelope json` prints one JSON object:
  the rendered document and, per cell in document order, its id, index, span,
  hash, type, status, output and reports.
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
<!-- hql:output hash=338061c564a2 status=ok -->
- Bearer tokens
- Session cookies
<!-- /hql:output -->
````

### Cells share values

A cell with an `id` exports its value under that name to every other cell of
the document, and nothing else it binds. The cells form a dependency graph:
they are checked as a whole, then evaluated so that each runs after the cells
it reads, document order breaking ties. A cycle and a duplicate id are errors
naming the cells, and a cell that reads a failed one is skipped, its output
saying which cell it waited on. An output's `hash` covers the cell's code and
the hashes of the cells it reads, so an output whose hash differs from the
cell's is stale, which a host tells without running anything
([HQL-0009](../../proposals/HQL-0009/README.md)).

````markdown
```hql#eval id=open
cards | filter(c => c.metadata.status == "todo")
```

```hql#eval
open | size
```
````

The output is Markdown, as [HQL-0008](../../proposals/HQL-0008/README.md)
requires of a document host. A cell's result is written in the `hmd`
presentation format: a record becomes a pipe table, a sequence a bulleted list,
a card a wikilink in that list, and a graph a fenced `d2` block. `markdown` and
`hmd` text is spliced in as it is, and text of another format is a fenced block
with that format's info string.

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
