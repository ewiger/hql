# Reporting

Status: implemented.

A run produces a structured log beside its value. Which entries stop it, and
which are shown, is a policy, not a property of the language, so it is
configured rather than fixed.

| Severity | Meaning | Example |
| --- | --- | --- |
| `debug` | A value the program captured to look at | `xs \| debug(label = "xs")` |
| `info` | A value or a fact the program states | `debug(x, level = "info")` |
| `warning` | Work continued and the result stands | a reference that does not resolve |
| `error` | The result is wrong, but more could still be examined | a type or name error |
| `failure` | Nothing further can be examined | the source did not parse; arithmetic overflowed |

## The queue comes back beside the result

Running a program yields a value *and* a queue of everything the run had to say,
oldest first. The queue is not an alternative to the result: a program that
warned still produced one, and the caller walks the queue afterwards.

```rust
let outcome = hql::run(source, &vault, Mode::Strict);
outcome.value      // Option<Value>
outcome.reports    // a queue: next_report(), iter(), worst(), count(severity)
```

A warning never changes an exit status. An error or a failure makes the result
absent and the status `1`.

## Modes

| Mode | When the run stops |
| --- | --- |
| `strict` | at the first error — the default |
| `collect` | only at a failure; every error findable is queued |

`strict` is the default because a wrong answer that keeps going is harder to
notice than one that stops. `collect` is for looking at a whole query at once:
every statement is checked even after one fails, and a statement that failed
still binds its name at the open tree type, so the statements after it are
reported on their own faults rather than on a cascade from this one.

A failure stops both modes, because collecting after an unparseable source has
nothing to collect.

## Where a mode comes from

The most specific layer that states one wins:

1. **the query** — `--report strict|collect`
2. **the vault** — `hql.toml` at its root, `[report] mode = "collect"`
3. **the user** — `HQL_REPORT` in the environment
4. **the default** — `strict`

## The shown level

`level` is the lowest severity a text host writes: `debug`, `info`, `warning` or
`error`, and `warning` by default. It comes from the same layers as the mode:
`--report-level`, then `[report] level` in the vault's `hql.toml`, then
`HQL_REPORT_LEVEL`, then the default. Hiding an entry does not discard it:
`Reports::shown(level)` filters a copy, and `--format json` carries every entry.

`hql config` prints the mode and the level in force and which layer supplied
each, so the precedence never has to be guessed.

## Entries

Every entry has `severity`, `stage`, `message` and `span`. An entry the program
captured with [`debug`](../../wiki/hql/operators/debug-op.hmd) also has `code`
(`debug`), `label` when one was given, `type`, `found` (the whole value as data)
and `trace`: each step on the path evaluation took to it, its one-based position
in its pipeline, and the element its function was applied to. The log survives
an evaluation failure, so what was captured before it is still reported.

## Rendering

A report carries a zero-based, half-open UTF-8 byte range. That is the
machine-readable form and stays that way; turning it into a line, a column and a
caret is a separate step in `render`, which is why it is testable rather than
something only the binary can do.

```text
type error: addition requires two Int or two Float operands
 --> <expression>:2:3
  |
2 |   true + 1
  |   ^^^^
```

`--format json` emits the same queue as data, with the fields above and the
byte span, for an editor or another host.

Tests: `tests/reporting.rs`, `tests/cli.rs`. See [the command line](cli.md).
