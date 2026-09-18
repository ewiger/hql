# Reporting

Status: implemented.

A run produces three streams, not one. Which of them stops it is a policy, not a
property of the language, so it is configured rather than fixed.

| Severity | Meaning | Example |
| --- | --- | --- |
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

`hql config` prints the mode in force and which layer supplied it, so the
precedence never has to be guessed.

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

`--format json` emits the same queue as data, with `severity`, `stage`,
`message` and the byte span, for an editor or another host.

Tests: `tests/reporting.rs`, `tests/cli.rs`. See [the command line](cli.md).
