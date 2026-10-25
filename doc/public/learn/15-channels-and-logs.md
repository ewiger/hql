# 15. Channels and logs

!!! note "The `io` and `io.logging` modules"

    This chapter covers channels from the `io` module and the `io.logging`
    module, both shipping in the next release.

An endpoint in [chapter 14](14-reading-and-writing.md) held bytes, and a format
said how they were documents. A **channel** holds values. A **logger** writes
structured events about a program to any endpoint, a channel included.

## Channels

`Channel<T>(name)` is an endpoint of values of type `T`, held in memory for one
evaluation. It keeps the values' type, so it takes no format. `target` writes
one value to it, `sink` writes each value of a sequence, and `source` gives
what it holds now:

<!-- run: skip -->
```hql
c = Channel<Int>("nums")
1 | target(c)
[2, 3] | sink(c)
source(c)
```

```text
[1, 2, 3]
```

Reading does not consume. Two reads give the same values:

<!-- run: skip -->
```hql
c = Channel<Int>("n")
[1] | sink(c)
a = source(c)
b = source(c)
[size(a), size(b)]
```

```text
[1, 1]
```

`tap(c)` gives only what is written after it, as a stream:

<!-- run: skip -->
```hql
c = Channel<Int>("n")
[1] | sink(c)
t = tap(c)
[2] | sink(c)
t | take(5)
```

```text
[2]
```

The element type is checked like any other:

<!-- run: skip -->
```hql
c = Channel<Int>("n")
"x" | target(c)
```

```text
type error: Channel<Int> holds Int, not String
 --> <expression>:2:7
  |
2 | "x" | target(c)
  |       ^^^^^^^^^
```

One name is one channel within a program run, or within one cell of a
document. A cell of a document being rendered may write a channel, because a
channel stays inside the evaluation.

## Logging

A program reports on itself through a logger, apart from its result.
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
has five operations, one per level, ordered from least to most severe: `trace`,
`debug`, `info`, `warn` and `error`. Each takes a message and, optionally,
structured data, and is called through a pipe or directly: `info(log, "x")` is
`log | info("x")`.

A logger writes events at its threshold or above, `Info` unless `level` says
otherwise, so the `debug` call above writes nothing. `--log-level` overrides
every threshold without changing the program:

```text
$ hql --log-level debug 'import logging
log = Logger(stderr, Text, name = "club")
log | debug("now shown")
1'
DEBUG club now shown
1
```

The format is the endpoint's, as for `sink`. In `Json` each event is one
document with four keys:

```text
{"data":{"title":"Dune"},"level":"Warn","logger":"club","message":"low stock"}
```

## Logs as values

A logger whose destination is a `Channel<LogEvent>` keeps its events as values,
which the rest of the program can query. A level is a type, so it is tested
with `is`:

<!-- run: skip -->
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
document.

## What is next

A run of a language model is one more thing a program can do, and its record
one more value. [Chapter 16](16-agents-and-runs.md) builds context from a vault,
runs an agent over it, and reads back everything that happened.
