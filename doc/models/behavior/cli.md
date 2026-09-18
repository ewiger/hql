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
| `hql render <file>` | run the HQL blocks in a document and transclude the answers |
| `hql builtins` | list the steps a pipeline may use |
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
than quietly finding nothing. That is the CLI's answer to `CON-08`: this host
requires a vault for card-ness. Whether the *language* does is still open.

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
is run against the vault and its answer written beneath it as an `hql#result`
block, which is replaced rather than appended next time — so rendering twice
leaves what rendering once did.

````markdown
```hql#eval
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
| table
```
````

A to-do list is then a query rather than a list somebody maintains. A collection
transcludes as a table, because a document wants the rows; a presenter the query
chose is used as written. `--write` puts the rendered document back over the
file.

This is the console implementation of the direction
[program values](program-values.md) records for `hql#eval`, `hql#query` and
`hql#declare`. A host that draws rather than prints — an editor extension
showing the result inline — consumes the same value; only the presentation
differs. `hql#declare` is not implemented, because contributing knowledge to a
store is the effect boundary the [knowledge model](../domain/knowledge.md)
records as undrawn.

Tests: `tests/cli.rs`, `tests/transclude.rs`.
