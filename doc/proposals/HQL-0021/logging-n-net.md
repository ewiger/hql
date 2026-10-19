# Logging and In-Memory Channels in HQL

> Companion note of [HQL-0021](README.md), which states the decision. The
> examples here are written in the record's spelling where it differs:
>
> | Here | In HQL-0021 |
> | --- | --- |
> | `Io.net.http(…)`, `use Io.logging.Logger` | `import net` then `http(…)`; `import logging` |
> | `channel<LogEvent>("logs")` | `Channel<LogEvent>("logs")`, a type's constructor |
> | `Logger(sink(stderr, Text), name = …)` | `Logger(stderr, Text, name = …)` |
> | `sink(file("run.log"), Json)` for a log kept across runs | `file("run.log").end`, which appends |
> | `log.info("…")` | `log \| info("…")`, or `info(log, "…")` |
> | `enum LogLevel`, `e.level == Error` | five subtypes of `LogLevel`, `e.level \| is(Error)` |

## Context

HQL is gaining a general I/O model based on endpoints and typed flow.

The current direction distinguishes finite/materialized I/O from streaming I/O:

```hql
source(stdin, Json)
source(file("birds.json"), Json)

value
| target(file("result.json"), Json)

stream
| sink(stdout, Json)
```

Streaming endpoints can be attached explicitly:

```hql
tap("events")
| filter(...)
| sink(stdout, Json)
```

The I/O extension namespace should be structured by concern:

```text
Io

Io.net
    http
    socket
    websocket
    ...

Io.logging
    Logger
    LogEvent
    LogLevel
```

Logging must support ordinary destinations such as:

```hql
stderr
file("hql.log")
http(...)
socket(...)
```

but must also support a host-visible in-memory destination.

This is especially useful when HQL executes inside HyperMarkdown cells. A cell has a primary result which is rendered or persisted as cell output, while diagnostic or progress information should be captured separately and optionally displayed by the host.

A logger therefore must not redefine output. It should use the same I/O abstractions as everything else.

---

# Decision

## 1. Logging is a structured event stream

The fundamental logging value is:

```hql
type LogEvent {
    level: LogLevel,
    logger: String,
    message: String,
    data: Data,
}
```

with:

```hql
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
```

A logger produces:

```text
Stream<LogEvent>
```

conceptually.

Logging destinations are sinks for these events.

A logger therefore does not own a filesystem writer, terminal writer, network client, or VS Code integration.

It owns:

1. a logger name,
2. a destination sink,
3. optional filtering/configuration.

---

# 2. Logger construction

The canonical form is:

```hql
log =
    Logger(
        sink(stderr, Text),
        name = "hql"
    )
```

The destination is first because it determines where the logger's events go.

The name identifies the logical producer, similar to named logger hierarchies in conventional logging systems.

Examples:

```hql
query_log =
    Logger(
        sink(stderr, Text),
        name = "query"
    )
```

Structured file logging:

```hql
query_log =
    Logger(
        sink(file("query.log"), Json),
        name = "query"
    )
```

Network logging:

```hql
audit_log =
    Logger(
        sink(
            Io.net.http(
                "https://logs.example.com/events",
                {method: "POST"}
            ),
            Json
        ),
        name = "audit"
    )
```

The logger is therefore independent of transport.

---

# 3. Logging operations

The initial API should expose the conventional operations:

```hql
log.trace("...")
log.debug("...")
log.info("...")
log.warn("...")
log.error("...")
```

Structured context can be supplied as a second argument:

```hql
log.info(
    "indexed cards",
    {
        count: cards | count,
        vault: vault.name,
    }
)
```

which emits a `LogEvent` conceptually equivalent to:

```hql
{
    level: Info,
    logger: "indexer",
    message: "indexed cards",
    data: {
        count: 412,
        vault: "docs",
    },
}
```

Logging is an `Effectful` operation.

The call:

```hql
log.info(...)
```

returns:

```text
Unit
```

It must never become part of the surrounding query result accidentally.

---

# 4. Logger names

Logger names are hierarchical strings.

For example:

```text
hql
hql.query
hql.query.typecheck
hql.io
hql.io.net
```

A logger may therefore be created as:

```hql
log =
    Logger(
        sink(stderr, Text),
        name = "hql.query.typecheck"
    )
```

The hierarchy is semantic rather than syntactic in HQL 1.0.

No special nested logger type is required.

A host or logging configuration may later apply filters such as:

```text
hql.query.* = Debug
hql.io.net  = Warn
```

without changing the language model.

---

# 5. In-memory channels

HQL introduces one general I/O primitive:

```hql
channel<T>(name)
```

A channel is an in-memory typed endpoint.

For example:

```hql
events =
    channel<LogEvent>("cell.logs")
```

Unlike files, stdout, HTTP, or sockets, a channel does not encode values into bytes.

It preserves `T`.

Therefore this is valid:

```hql
sink(events)
```

without a format argument.

Likewise:

```hql
source(events)
```

does not decode a format.

A typed channel is therefore an endpoint at the HQL value layer rather than the byte layer.

---

# 6. Channels are not Go synchronization channels

The name is intentionally borrowed from the idea of channels, but HQL channels are not initially concurrency synchronization primitives.

In particular, they do not imply:

- blocking rendezvous,
- goroutine semantics,
- ownership transfer,
- one sender / one receiver,
- deadlock semantics.

The initial abstraction is simpler:

> A channel is a host-scoped in-memory sequence of typed values that can receive values and later expose them to the host or to another HQL pipeline.

This is sufficient for logging, cell output capture, testing, instrumentation, and future agent/event plumbing.

Concurrency semantics may be introduced separately if HQL ever needs them.

---

# 7. FIFO is the default

The default channel is ordered FIFO:

```hql
logs =
    channel<LogEvent>("cell.logs")
```

is equivalent conceptually to:

```text
Channel<LogEvent, Fifo>
```

Events are observed in write order.

This is the only channel ordering required for the first implementation.

LIFO and other queue disciplines may be introduced later, for example:

```hql
channel<Data>("work", order = Lifo)
```

but they should not be part of the logging MVP.

Logging depends on chronological ordering, so FIFO should remain the default.

---

# 8. Logging into a channel

A cell-local logger can therefore be defined as:

```hql
cell_logs =
    channel<LogEvent>("cell.logs")

log =
    Logger(
        sink(cell_logs),
        name = "analysis"
    )
```

Then:

```hql
log.info("starting analysis")

result =
    cards
    | filter(c => c.metadata.category == "bird")

log.info(
    "analysis completed",
    {count: result | count}
)

result
```

The program's result remains:

```text
result
```

while `cell_logs` contains:

```text
Info  analysis  starting analysis
Info  analysis  analysis completed  {count: ...}
```

The two outputs are separate by construction.

---

# 9. `source(channel)` and `tap(channel)`

The existing I/O terminology gives channels two useful observation modes.

## Snapshot

```hql
source(cell_logs)
```

returns the values currently buffered:

```text
Seq<LogEvent>
```

It does not wait for future events.

This is useful after execution:

```hql
logs =
    source(cell_logs)
```

The operation should be non-destructive by default.

Reading diagnostics should not make them disappear.

---

## Live observation

```hql
tap(cell_logs)
```

returns:

```text
Stream<LogEvent>
```

and observes events as they arrive.

For example:

```hql
tap(cell_logs)
| filter(e => e.level == Error)
| sink(stderr, Text)
```

This enables live forwarding and interactive interfaces.

The distinction follows the general HQL vocabulary:

```text
source(channel)   current materialized contents
tap(channel)      ongoing event stream
```

---

# 10. `target` and `sink` with channels

The same complementary distinction applies on output.

One value can be appended:

```hql
event
| target(cell_logs)
```

while a stream can be connected continuously:

```hql
events
| sink(cell_logs)
```

For logging, the logger internally behaves as a producer into:

```hql
sink(cell_logs)
```

because log entries are naturally incremental.

No `ChannelWriter`, `ChannelReader`, `QueueWriter`, or similar types are needed.

---

# 11. Channel lifetime belongs to the host

A channel is meaningful only inside an execution scope.

The host creates and owns that scope.

For CLI evaluation:

```bash
hql run program.hql
```

channels normally live for the duration of the program.

For a HyperMarkdown cell:

```text
cell execution
    ├── primary result
    └── named channels
```

channels live for the duration of that cell execution and remain available to the host while it processes the result.

The VS Code extension can therefore execute a cell and receive something conceptually like:

```text
ExecutionResult {
    value: Data,
    channels: Map<String, Seq<Data>>,
}
```

This is a host/runtime protocol, not a new HQL language value.

The cell still returns exactly one ordinary HQL value.

---

# 12. HyperMarkdown cell integration

The normal cell output remains unchanged.

For example:

````markdown
```hql#eval
cards
| filter(c => c.metadata.category == "bird")
| map(c => c.title)
```
````

produces the normal cell result.

A cell may additionally request one or more channels to be displayed.

Conceptually:

````markdown
```hql#eval log=cell.logs
...
```
````

The exact fence parameter syntax belongs to the HyperMarkdown cell specification, not the HQL logging extension.

The important contract is:

```text
log=cell.logs
```

means:

> After or during cell execution, expose the named in-memory channel as auxiliary cell output.

This allows the rendered cell to have two clearly distinct regions:

```text
Result
────────────────────
owl
eagle
hawk


Logs
────────────────────
INFO  loading cards
INFO  312 cards loaded
DEBUG filtering category=bird
```

The primary HQL value and logger output remain semantically independent.

---

# 13. Logs should normally not be persisted into the card

The current cell result can be saved into HyperMarkdown so that a document remains meaningful without executing the cell.

Logs have different semantics.

They are generally:

- diagnostic,
- execution-specific,
- transient,
- potentially verbose,
- potentially environment-dependent.

Therefore the recommended default is:

```text
cell result   persisted
log channels  ephemeral
```

The VS Code extension may display captured logger channels after execution without writing them into the `.hmd` source.

Persisting a log channel should require explicit cell configuration.

This prevents documents from accumulating stale execution noise.

---

# 14. Host-supplied channels

For the most common cell use case, the document should not need boilerplate such as:

```hql
cell_logs = channel<LogEvent>("cell.logs")
```

The execution host may pre-bind named channels.

For example, the HyperMarkdown cell host could provide:

```text
cell.logs
```

as a channel endpoint.

Then the program only needs:

```hql
log =
    Logger(
        sink(cell.logs),
        name = "analysis"
    )
```

This is analogous to the host already supplying values such as:

```hql
stdin
stdout
stderr
vault
```

However, `cell.logs` is host-specific.

Portable HQL code should be free to create its own channel:

```hql
logs =
    channel<LogEvent>("logs")
```

---

# 15. Default logger destination

`Io.logging` should not introduce an invisible global logger.

This should not happen:

```hql
info("something")
```

with an implicit process-wide destination.

Explicit logger construction is preferred:

```hql
log =
    Logger(
        sink(stderr, Text),
        name = "app"
    )
```

or, in a cell:

```hql
log =
    Logger(
        sink(cell.logs),
        name = "analysis"
    )
```

A host may provide a logger binding if desired:

```hql
log.info("...")
```

but that is a host binding, not language magic.

This follows the same principle as `stdin` and `vault`.

---

# 16. Formatting

A byte-oriented sink requires a format:

```hql
Logger(
    sink(stderr, Text),
    name = "query"
)
```

or:

```hql
Logger(
    sink(file("query.jsonl"), Json),
    name = "query"
)
```

Because the logger emits a stream, JSON naturally becomes one JSON document per log entry:

```json
{"level":"Info","logger":"query","message":"starting","data":{}}
{"level":"Debug","logger":"query","message":"resolved source","data":{"cards":42}}
```

No `JsonLines` format is required.

A channel is typed and therefore requires no serialization:

```hql
Logger(
    sink(cell.logs),
    name = "query"
)
```

This preserves the full `LogEvent` structure for the VS Code renderer.

The renderer can decide whether to show it as text, a table, collapsible structured data, or some future richer representation.

---

# 17. Multiple destinations

The logger itself should initially have one sink.

Fan-out already belongs to the stream model and should not be reinvented by logging.

If multiple destinations become important, they should be composed at the I/O layer.

Conceptually:

```text
LogEvent stream
    ├── sink(stderr, Text)
    ├── sink(file("hql.log"), Json)
    └── sink(cell.logs)
```

A future generic fan-out sink could implement this.

Do not add:

```hql
Logger(
    outputs = [...]
)
```

until the underlying I/O system has a general mechanism for fan-out.

Logging should not solve an I/O problem privately.

---

# 18. Filtering

Log level filtering should occur before an expensive destination when possible.

For example:

```hql
Logger(
    sink(stderr, Text),
    name = "query",
    level = Info
)
```

means that:

```hql
log.debug(...)
```

does not reach the sink.

The host may override thresholds.

This makes cell execution especially useful:

```text
normal run     Info
debug run      Debug
CI             Warn
```

without modifying the HQL program.

Logger level is therefore configuration, not part of `LogEvent`.

---

# 19. Effect model

Constructing an endpoint is pure:

```hql
channel<LogEvent>("logs")
file("hql.log")
Io.net.http("https://...")
```

Constructing a logger is also pure:

```hql
Logger(
    sink(cell.logs),
    name = "query"
)
```

until it is used.

Emitting a log event is effectful:

```hql
log.info("...")
```

because it causes a value to enter the associated sink.

The effect capability derives from the sink.

For example:

```hql
Logger(sink(cell.logs), ...)
```

requires only in-memory channel output.

```hql
Logger(sink(stderr, Text), ...)
```

requires host stderr output.

```hql
Logger(sink(file("hql.log"), Json), ...)
```

requires filesystem write permission.

```hql
Logger(sink(Io.net.http(...), Json), ...)
```

requires network output permission.

Logging therefore does not introduce a special security model.

It inherits the I/O effect model.

---

# 20. HyperMarkdown security

This distinction is important for cells.

A trusted cell may be allowed:

```hql
Logger(
    sink(cell.logs),
    name = "analysis"
)
```

while still being denied:

```hql
Logger(
    sink(file("somewhere.log"), Text),
    name = "analysis"
)
```

and:

```hql
Logger(
    sink(Io.net.http("https://..."), Json),
    name = "analysis"
)
```

The host can therefore grant:

```text
memory channel write     allowed
filesystem write         denied
network write            denied
```

A logging API does not become a way around the normal HQL capability rules.

---

# 21. Namespace layout

The proposed namespace structure is:

```text
Io
├── source
├── target
├── tap
├── sink
├── file
├── channel
│
├── net
│   ├── http
│   ├── websocket
│   └── socket
│
└── logging
    ├── Logger
    ├── LogEvent
    └── LogLevel
```

This separation is intentional.

`Io.net` introduces transports.

`Io.logging` introduces structured producers of events.

`channel` stays in `Io` itself because it is a generic endpoint useful far beyond logging.

---

# 22. Example: CLI logging

```hql
use Io.logging.Logger

log =
    Logger(
        sink(stderr, Text),
        name = "birds"
    )

log.info("loading input")

birds =
    source(stdin, Json)
    | filter(b => b.count > 2)

log.info("filter completed")

birds
| sink(stdout, Json)
```

Run:

```bash
cat birds.jsonl | hql run birds.hql
```

stdout:

```json
{"species":"owl","count":4}
{"species":"eagle","count":7}
```

stderr:

```text
INFO birds loading input
INFO birds filter completed
```

The data pipeline and diagnostics are completely separate.

---

# 23. Example: structured logging to a file

```hql
log =
    Logger(
        sink(file("./run.log"), Json),
        name = "import"
    )

log.info(
    "import started",
    {source: "birds.csv"}
)

result =
    source(file("birds.csv"), Csv)
    | map(normalize)

log.info(
    "import completed",
    {rows: result | count}
)

result
```

`run.log` contains a stream of JSON `LogEvent` documents.

---

# 24. Example: HyperMarkdown cell

A cell can use a host-provided channel:

```hql
log =
    Logger(
        sink(cell.logs),
        name = "bird-report"
    )

log.info("query started")

birds =
    cards
    | filter(c => c.metadata.category == "bird")

log.debug(
    "cards selected",
    {count: birds | count}
)

birds
| map(c => c.title)
```

The primary result is:

```text
Barn Owl
Golden Eagle
Robin
```

while `cell.logs` contains structured diagnostic entries.

The VS Code extension can render them below or beside the ordinary result.

---

# 25. Example: consuming logs as HQL data

Because the channel contains ordinary typed HQL values, logging output can itself be queried:

```hql
source(cell.logs)
| filter(e => e.level == Error)
| map(e => {
    logger: e.logger,
    message: e.message,
})
```

Or observed live:

```hql
tap(cell.logs)
| filter(e => e.level == Error)
| sink(stderr, Text)
```

This is an important property.

Logs are not special opaque strings.

They are ordinary HQL data.

---

# 26. Consequences

The model creates the following clean relationships:

```text
                         HQL I/O

                       Endpoint
                           │
          ┌────────────────┼────────────────┐
          │                │                │
        file             net.*           channel<T>
          │                │                │
        bytes            bytes           typed values
          │                │                │
          └──────── source / tap ───────────┘
                           │
                        HQL values
                           │
          ┌──────── target / sink ──────────┐
          │                │                │
        bytes            bytes           typed values
```

Logging sits on top:

```text
Logger
   │
   │ produces LogEvent
   ▼
sink
   │
   ├── stderr
   ├── file
   ├── Io.net.*
   └── channel<LogEvent>
```

There is no logging-specific writer infrastructure.

---

# 27. Rejected alternatives

## Special logger files

Rejected:

```hql
Logger.file("run.log")
```

This duplicates `Io.file` and bypasses the endpoint model.

---

## Special logger network transport

Rejected:

```hql
Logger.http(...)
```

Networking belongs in:

```text
Io.net
```

---

## String-only log streams

Rejected:

```text
Stream<String>
```

as the fundamental representation.

Logging should preserve structured data as `LogEvent`.

Formatting to text happens only at byte-oriented sinks.

---

## Global implicit logger

Rejected:

```hql
log("hello")
```

where the destination is invisible.

HQL should preserve explicit effects and host-controlled capabilities.

---

## Go-style synchronization semantics

Rejected for the initial channel implementation.

The current requirement is an in-memory typed output buffer and observable event stream, not a general CSP concurrency system.

---

# 28. Minimal implementation scope

The first implementation should include only:

```text
Io.channel<T>

Io.logging.Logger
Io.logging.LogEvent
Io.logging.LogLevel

Logger.info
Logger.warn
Logger.error
Logger.debug
```

and interoperability with:

```text
source(channel)
tap(channel)

target(channel)
sink(channel)
```

plus:

```text
stderr
file(...)
```

as initial logger destinations.

The HyperMarkdown/VS Code host then needs only one additional capability:

> Capture selected named channels produced during HQL cell execution and render them as auxiliary cell output.

HTTP logging, WebSocket logging, custom queue ordering, logger hierarchies, rolling files, and persistence policies can follow later without changing the core abstraction.

---

# Decision summary

The logging feature should not create another output system.

A logger is:

```hql
Logger(
    sink,
    name = "..."
)
```

and emits typed:

```text
LogEvent
```

values.

An in-memory:

```hql
channel<LogEvent>("...")
```

is a generic HQL endpoint that can be written through `target`/`sink` and observed through `source`/`tap`.

For HyperMarkdown cells, the execution host may expose a channel such as:

```hql
cell.logs
```

and the VS Code extension may render that channel separately from the cell's primary persisted result.

This preserves one coherent model:

```text
source / target     materialized I/O
tap / sink          streaming I/O
file / net / channel
                    endpoint kinds
Logger              structured LogEvent producer
HyperMarkdown       host that chooses which channels to expose
```