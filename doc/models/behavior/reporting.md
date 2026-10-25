# Reporting

Status: implemented.

A run produces a value and the contents of its **diagnostic channel**
([HQL-0045](../../proposals/HQL-0045/README.md)). What a diagnostic is, its
four axes and its fields, is the
[error-handling card](../../wiki/hql/error-handling.hmd). This model is the
host's side: the channel, the outcome, the mode and level, and delivery.

## The channel and the outcome

The runtime owns one `Channel<Diagnostic>` per run, on the `io` host. It is
the mechanism of every channel, a bounded FIFO of committed values, owned by
the runtime rather than the program:

| Operation | Meaning |
| --- | --- |
| `io::open_diagnostics()` | opens the channel unless a run holds it; answers whether this call opened it |
| `io::write_diagnostic(d)` | commits `d`, then tells each tap; with no channel open, it goes nowhere |
| `io::tap_diagnostics(f)` | `f` observes each diagnostic as it is committed; a tap does not keep the run open |
| `io::diagnostics_written()` | the contents, in commit order |
| `io::close_diagnostics()` | detaches the taps, closes the channel and gives its contents |

`hql::run_on` opens the channel unless the host holds it, writes the
loader's diagnostics a vault carries when it opened it, then the parser's,
the checker's and the evaluator's, each stamped with its phase:

```rust
let outcome = hql::run(source, &vault, Mode::Strict);
outcome.value        // Option<Value>
outcome.inferred     // Option<TypeRef>
outcome.diagnostics  // Vec<Diagnostic>, the channel's contents in order
outcome.failed()     // a diagnostic at or above `error`
```

The CLI holds the channel for the whole run: it opens it before loading the
vault, so the loader's `skipped-document` is written first, and closes it
after delivering. `hql repl` gives each line typed its own run and channel,
and delivers the loader's diagnostics once, before the first.

A warning never changes an exit status. A diagnostic at or above `error`
makes the status `1`.

## Modes

`mode` is how much continuation the run asks its phases for; whether a phase
continues is the phase's.

| Mode | Meaning |
| --- | --- |
| `strict` | decline optional continuation after the first `error` — the default |
| `collect` | ask every phase for as much continuation as it can give |

Under `collect` the checker checks every statement, a failed one still bound
at the open tree type, and writes each diagnostic `recoverable`; `map`,
`filter` and `flat_map` step past an element whose function failed, unless the
failure is an overflow or an I/O failure. A parse failure is `fatal` under
both.

## Where a setting comes from

The most specific layer that states one wins:

1. **the query** — `--report strict|collect`, `--report-level LEVEL`
2. **the vault** — `[report] mode` and `level` in `hql.toml` at its root
3. **the user** — `HQL_REPORT`, `HQL_REPORT_LEVEL` in the environment
4. **the default** — `strict`, and `warning`

`level` is the lowest severity delivered to the default target. Filtering
does not discard: the channel holds every diagnostic, and `--envelope json`
carries them all. `hql config` prints the mode and the level in force and the
layer that supplied each.

## Delivery

Delivery is a format and an endpoint. The default wiring is
`diagnostics | DiagnosticText | stderr`, with the value on `stdout`:

```text
check error[type-mismatch]: addition requires two Int, two Float or two Decimal operands
 --> <expression>:2:3
  |
2 |   true + 1
  |   ^^^^
```

`render::diagnostic_text` writes one diagnostic: the heading
`phase severity[kind]: message`, the location, the source line and a caret
when the host has the source text, then one `=` line per field that applies,
`cell`, `note`, `help`, `expected`, `found`, `element`, `source`, `bindings`
and `trace`. `render::report` writes the channel's contents and a count per
severity when there is more than one. The byte span stays the
machine-readable form; the line and column are a presentation.

| Option | Delivery |
| --- | --- |
| `--diagnostics-format text` | `DiagnosticText` on standard error — the default |
| `--diagnostics-format json` | one JSON document per diagnostic per line on standard error |
| `--envelope json` | the channel's contents under `diagnostics`, beside `origin`, `type` and `value` |

The host fills the `origin` a diagnostic leaves unnamed with what the program
was read from, and puts the document's path before a cell's `#cell`.

Tests: `tests/reporting.rs`, `tests/cli.rs`, and the sessions under
[`examples/error-handling/`](../../../examples/error-handling/README.md). See
[the command line](cli.md).
