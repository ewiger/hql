<!-- hql reference: std/io/logging.Logger -->
# `Logger`

In [`std/io/logging`](../index.md).

```hql
type Logger { name : String level : LogLevel // the least severe level it writes }
```

A producer of events: its `name` and the least severe `level` it
writes. Its destination is not a field: what a logger writes to is
visible only where it is constructed.

```hql
fn Logger<F : Format, L : LogLevel>(to: Writable, format: Type<F>, *, name: String, level: Type<L> = Info) : Logger
```

A `Logger` named `name` that writes to `to` in `format` each event at
least as severe as `level`, `Info` unless written. The format is omitted
for a `Channel<LogEvent>`, which takes the events as values.
Construction is pure and writes nothing:
`Logger(stderr, Text, name = "birds")`.
<!-- hql reference: end -->
