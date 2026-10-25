<!-- hql reference: std/io/logging -->
# `std/io/logging`

```hql
module io.logging
```

Logging: structured events written to a destination, apart from the
result. A logger owns no writer: it names a producer and holds a
destination, as `sink` would write it (module `io`): `stderr` or a file
in a format, a network endpoint, or a `Channel<LogEvent>` that keeps the
events as values for the host. A program imports the module:
`import logging`.

- [`LogLevel`](types/LogLevel.md) — How severe an event is.
- [`Trace`](types/Trace.md) — The least severe `LogLevel`, below `Debug`.
- [`Debug`](types/Debug.md) — The `LogLevel` above `Trace` and below `Info`.
- [`Info`](types/Info.md) — The `LogLevel` above `Debug` and below `Warn`; a logger's threshold unless written.
- [`Warn`](types/Warn.md) — The `LogLevel` above `Info` and below `Error`.
- [`Error`](types/Error.md) — The most severe `LogLevel`, above `Warn`.
- [`LogEvent`](types/LogEvent.md) — One logged event: its `level`, the `logger` that produced it, its `message` and its `data`.
- [`Logger`](types/Logger.md) — A producer of events: its `name` and the least severe `level` it writes.
- [`trace`](trace.md) — Writes one `Trace` event with `message` and `data` to `log`'s destination when `log`'s threshold is `Trace`, and gives `Unit`.
- [`debug`](debug.md) — Writes one `Debug` event with `message` and `data` to `log`'s destination when `log`'s threshold is at most `Debug`, and gives `Unit`.
- [`info`](info.md) — Writes one `Info` event with `message` and `data` to `log`'s destination when `log`'s threshold is at most `Info`, and gives `Unit`.
- [`warn`](warn.md) — Writes one `Warn` event with `message` and `data` to `log`'s destination when `log`'s threshold is at most `Warn`, and gives `Unit`.
- [`error`](error.md) — Writes one `Error` event with `message` and `data` to `log`'s destination, whatever `log`'s threshold, and gives `Unit`.
- [`Cell`](types/Cell.md) — The cell `hql render` is running, bound as `cell`.
<!-- hql reference: end -->
