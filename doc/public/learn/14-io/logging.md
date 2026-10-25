# Logging

A program reports on itself through a **logger**, apart from its result.
`import logging` provides one. `Logger(endpoint, Format, name = …)` names a
producer and the destination it writes to:

<!-- run: skip -->
```hql
import logging

log = Logger(stderr, Text, name = "club")
books = cards | filter(c => c.metadata.category == "book")
log | info("selected", {books: books | size})
log | debug("not shown")
books | size
```

```text
INFO club selected {books: 7}
7
```

The event goes to standard error and the result to standard output. A logger
has five operations, one per level, ordered from least to most severe:
`trace`, `debug`, `info`, `warn` and `error`. Each takes a message and,
optionally, structured data, and is called through a pipe or directly:
`info(log, "x")` is `log | info("x")`. Each gives `Unit`, so a log call never
becomes part of a result.

## The threshold

A logger writes events at its threshold or above, `Info` unless `level` says
otherwise, so the `debug` call above writes nothing. A level is a type, and
the threshold is passed as one:

```hql
import logging

log = Logger(stdout, Text, name = "club", level = Warn)
log | info("quiet")
log | warn("loud")
1
```

```text
WARN club loud
1
```

`--log-level` overrides every threshold without changing the program:

```text
$ hql --log-level debug eval 'import logging
log = Logger(stderr, Text, name = "club")
log | debug("now shown")
1'
DEBUG club now shown
1
```

## The event and its format

A logger writes as `sink` would: one document per event, in the format
given. In `Text` an event is one line: the level in capitals, the logger's
name, the message, then the data unless it is empty. In `Json` each event is
one document with four keys:

```hql
import logging

log = Logger(stdout, Json, name = "club")
log | warn("low stock", {title: "Dune"})
1
```

```text
{"data":{"title":"Dune"},"level":"Warn","logger":"club","message":"low stock"}
1
```

A log file is appended to by naming its end,
`Logger(file("app.log").end, Text, name = "club")`; a logger on `file(p)`
replaces the file with this run's events, by the rules of `sink`.

## Logs as values

A logger whose destination is a `Channel<LogEvent>` takes no format and keeps
its events as values, which the rest of the program can query. A level is a
type, so it is tested with `is`:

```hql
import logging

events = Channel<LogEvent>("events")
log = Logger(events, name = "club", level = Debug)
log | debug("loaded")
log | warn("low stock", {title: "Dune"})
log | error("missing", {title: "Ulysses"})

source(events)
| filter(e => (e.level | is(Warn)) or (e.level | is(Error)))
| map(e => {level: e.level, message: e.message})
```

```text
[{level: "Warn", message: "low stock"}, {level: "Error", message: "missing"}]
```

Inside a document, the host binds `cell`, the cell being rendered.
`Logger(cell.logs, name = …)` writes to that cell's own channel, whose events
an editor shows beside the cell's result. They are never saved into the
document: the cell's result is its value, unchanged by anything it logged.

Constructing a logger is pure. What a logger writes to is checked by the
host's policy as a `sink` to that endpoint, so logging grants nothing a
program could not do with `sink` ([effects and what a host allows](hosts.md)).
