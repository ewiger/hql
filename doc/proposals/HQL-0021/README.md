# HQL-0021: I/O: endpoints, channels, the network and logging

- **Status**: accepted, implemented in part
- **Created**: 2026-10-19
- **Source**: [design outline](design-outline.md),
  [logging and channels](logging-n-net.md),
  [issue 0015](../../issues/0015-jq-parity-analysis.md),
  [HQL-0012](../HQL-0012/README.md), [HQL-0017](../HQL-0017/README.md)

## Companion notes

- [`design-outline.md`](design-outline.md) — the worked examples of
  endpoints, `source`, `tap`, `target` and `sink`, and of network endpoints.
- [`logging-n-net.md`](logging-n-net.md) — the worked examples of loggers,
  log events and in-memory channels, and how a cell shows its logs.

The notes are the worked examples this record was drawn from. Where a note and
this record differ, this record holds: `JsonLines` and `Lines` are formats of
their own, and the prelude module is `std/io/mod.hql`.

## Abstract

An **endpoint** is where values come from or go to. `stdin`, `stdout` and
`stderr` are endpoints the host binds; `file(path)` constructs one, and so do
`http`, `websocket` and `socket` from `import net`; none of them performs I/O.
Four operations give an endpoint's bytes typed meaning in one of the existing
format types. `source(endpoint, Format)` reads it as the values it holds: a
`Seq` when the endpoint can be read again, a `Stream` when it cannot.
`tap(endpoint, Format)` reads it as a `Stream`, one value at a time, whatever
the endpoint. `target(endpoint, Format)` writes one completed value, and
`sink(endpoint, Format)` writes a stream piece by piece as it arrives. A
format decodes and encodes **documents**: an ordinary JSON file is one
document, a JSON Lines file is many, and a JSON array is one document. Whether
the documents end is carried by `Seq` and `Stream`, never by the format; how
they are framed is the format's, so `JsonLines`, one JSON document per line,
is a format of its own (HQL-0024's
[formats and codecs](../HQL-0024/parts/07-formats-and-codecs.md)). The expected type of a read decodes each document as a declared
record. A **channel**,
`Channel<T>(name)`, is an endpoint of values held in memory for one execution
scope; it keeps `T`, so it takes no format, and `source` gives what it holds
now while `tap` follows it. **Logging**, from `import logging`, is a stream of
`LogEvent` values that a `Logger` writes to one endpoint, so a log goes to
`stderr`, a file, the network or a channel by the same rules as any output.
Every read and write is `Effectful`, and the host decides at check time which
endpoints a program may use: `hql render` grants reading files beside the
document and writing channels, and refuses the rest. `source("id")` of
[HQL-0012](../HQL-0012/README.md) stays the form for a configured source. On
the command line, `hql '<program>'` is `hql eval '<program>'`, `--to json`
pretty-prints as `jq .` does, and a program whose result is `Unit` prints
nothing.

## Motivation

- `hql eval` evaluates a closed expression. Standard input is never read as
  data and a program cannot open a file, so `curl … | hql '…'` cannot do what
  `jq` does.
- [HQL-0012](../HQL-0012/README.md) reads configured sources by id. A one-off
  program over a file next to it, or a URL, should not need an entry in
  `hql.toml`.
- A program has exactly one output, its result, and the host writes it. A
  program cannot write a file, and cannot write two outputs.
- A program has no place for diagnostics. A cell in a document has one result,
  which is saved; what it reports along the way should be shown beside the
  result and not saved with it.
- Reading, writing, streaming and logging should be one model. Separate
  `input`, `read`, `write` and logger-file functions would each need their own
  typing, effect and security rules, and would not connect to data sources or
  to `Stream`.

## Non-goals

- **No HTTP request with a typed response.** `target` to an `http` endpoint
  sends a request and does not read the response. A request whose response is
  data is a configured source or a later record.
- **No framing beyond the formats.** A socket's documents are separated as
  their format separates them; length prefixes and delimiters of a protocol's
  own are not provided.
- **No concurrency in channels.** A channel is an ordered buffer: no
  rendezvous, no blocking, no single-reader rule. Orders other than first in,
  first out are not provided.
- **No logger fan-out and no global logger.** A logger has one destination,
  and a program logs only through a logger it was given or constructed.
- **No writes to configured sources.** `target("id")` and `sink("id")` need
  HQL-0012's sources to declare that they accept writes, which they do not
  yet.
- **No directory listing, deletion, renaming or file metadata.**
- **No `Result` type.** A failed read or write is an evaluation error, as a
  failed source fetch is.

## Specification

### 1. Modules

The declarations live in three modules of the standard library.

| Module | File | Provides | In a program |
| --- | --- | --- | --- |
| `io` | `std/io/mod.hql` | endpoints, channels, sources, targets | the prelude |
| `io.net` | `std/io/net/mod.hql` | `Http`, `WebSocket`, `Socket` | `import net` |
| `io.logging` | `std/io/logging/mod.hql` | `LogLevel`, `LogEvent`, `Logger` | `import logging` |

- `io` MUST be in the prelude, so `source(stdin, Json)` needs no import.
- `net` and `logging` MUST be imported ([HQL-0001](../HQL-0001/README.md)).
  Network access is then visible in the program's first lines.
- The standard library MUST name a module by its path below `std/`, folders
  joined by `.`, and a folder's `mod.hql` by the folder.
- Every operation of the three modules MUST be declared in its module as a
  bodyless header the extension implements, `effectful fn` for a read, a
  write and a log call, and `fn` for a constructor
  (HQL-0024's [contracts and binding](../HQL-0024/parts/12-contracts-and-binding.md)).
  Binding refuses an extension whose own marking differs.

### 2. Endpoints

An endpoint describes where values are; constructing one performs no I/O.

```hql
abstract type Endpoint
abstract type Readable   : Endpoint
abstract type Replayable : Readable   // the same contents on every read in one evaluation
abstract type Writable   : Endpoint

type Stdin  : Readable
type Stdout : Writable
type Stderr : Writable
type FileEnd : {Readable, Writable} {
    path : String
}
type File   : {Replayable, Writable} {
    path : String
    end  : FileEnd
}

stdin  : Stdin    // bound by the host
stdout : Stdout   // bound by the host
stderr : Stderr   // bound by the host
fn file(path: String) : File
```

- `stdin`, `stdout` and `stderr` MUST be ordinary bindings the host provides,
  as it provides `vault` ([HQL-0020](../HQL-0020/README.md)). A program that
  binds its own `stdin` shadows it.
- Every endpoint constructor MUST be `Pure`. It checks nothing about what it
  names, and the checker MUST NOT open, stat, resolve or connect to one.
- An endpoint's capabilities MUST be its supertypes, so the checker knows
  whether it is readable, writable and replayable. `source(stdout, Json)` and
  `42 | target(stdin, Json)` MUST be check errors naming the missing
  capability.
- A relative `file` path MUST resolve against the directory of the source unit
  that contains the call: the document for `hql render`, the program file for
  `hql run`, and the working directory for `hql eval` and `hql repl`. A
  document or program then reads the same file wherever it is run from.
- A `file` path with a scheme, `https://…`, MUST be refused, naming `http`.

A file is two endpoints: the whole file, and its end. Where a write lands is
which endpoint it names, so a file has no open mode.

| | `file(p)` | `file(p).end` |
| --- | --- | --- |
| `source` | every document in the file, as a `Seq` | refused: the end holds nothing yet |
| `tap` | the documents one at a time, then it ends | the documents written from now on, as `tail -f` |
| `target`, `sink` | replace the file | append to the file |

```hql
report | target(file("report.json"))             // rewrite
events | sink(file("events.jsonl").end)           // append
tap(file("app.log").end, Lines)                   // follow
| filter(line => contains(line, "ERROR"))
```

- `file(p).end` MUST be a `FileEnd`: `Readable` and `Writable`, and not
  `Replayable`, so `source` over it MUST be a check error naming `tap`.
- `tap(file(p).end, F)` MUST start at the file's end as it is when the tap
  begins, and give each document appended after it as it is written. It
  stops when the evaluation does. A missing file MUST be waited for, not
  refused, as `tail -F` does.
- A write to `file(p).end` MUST add after what the file holds and change
  nothing before it; a missing file MUST be created empty first.
- Reading and writing a file are chosen by the operation, and replacing and
  appending by the endpoint. Nothing else selects how a file is opened.

### 3. Formats decode and encode documents

A format reads a document from bytes and writes a document to bytes. An
endpoint may hold one document or a sequence of them.

| Format | One document reads as | Documents are separated by |
| --- | --- | --- |
| `Json` | `Data` | whitespace between top-level values |
| `Yaml` | `Data` | a `---` line |
| `Csv` | `Data`, one row keyed by the header | a line; the header is read once |
| `Lines` | `String`, one line without its terminator | a line terminator |
| `JsonLines` | `Data`, one JSON document per line | a line terminator |
| `Text`, `Markdown`, `Hmd`, `Hql` | `String`, the whole text, terminators included | not separable |

- A format MUST be a type narrowing `Format` (`std/core/mod.hql`), declared in
  `std/format/mod.hql`, and what it decodes MUST be the `Decoder<T>` it
  implements there: `Data` for `Json`,
  `JsonLines`, `Yaml` and `Csv`, `String` for the text formats.

- The documents of an endpoint MUST be read in the order they appear. These
  are the pieces [HQL-0017](../HQL-0017/README.md) writes when it formats a
  `Stream`, so `sink(e, F)` and `source(e, F)` are inverses.
- A JSON array MUST be read as one document. Spreading its members into the
  sequence is `emit` ([HQL-0007](../HQL-0007/README.md)) over documents that
  are sequences: `source(stdin, Json) | of_type(Seq<Data>) | emit`.
- A CSV cell MUST be read as a `String`, unless the expected type says
  otherwise (section "Typed decoding").
- Bytes that do not decode in their format MUST be an evaluation error naming
  the endpoint, the line and the column.
- `D2` and `Mermaid` MUST NOT be read. They are written by `target` only.
- A byte endpoint MUST be given a format. A channel MUST NOT be: it holds
  values, not bytes (section "Channels").

When the format argument is omitted, a literal path's extension implies it:

| Extension | Format |
| --- | --- |
| `.json` | `Json` |
| `.jsonl`, `.ndjson` | `JsonLines` |
| `.yaml`, `.yml` | `Yaml` |
| `.csv` | `Csv` |
| `.txt`, `.log` | `Lines` |
| `.md` | `Markdown` |
| `.hmd` | `Hmd` |
| `.hql` | `Hql` |
| `.d2` | `D2` |
| `.mmd` | `Mermaid` |

- The format MAY be omitted only when the endpoint is `file(p)` with `p` a
  string literal whose extension is in the table. Otherwise omitting it MUST
  be a check error naming the format argument. The type of what is read
  depends on the format, and the checker cannot read a path computed at run
  time.
- `.jsonl` and `.ndjson` MUST imply `JsonLines`, and `.txt` and `.log` MUST
  imply `Lines`: the framing is the format's, one record per line. `Json`
  over the same file reads the same documents, since whitespace separates
  top-level JSON values.
- A given format MUST win over the extension: `source(file("notes.data"),
  Json)` and `target(file("result.txt"), Json)` are allowed.

### 4. `source`

`source` reads an endpoint as the values it holds.

```hql
abstract type DataSource
abstract type FiniteSource<T> : {DataSource, Seq<T>}
abstract type LiveSource<T>   : {DataSource, Stream<T>}

effectful fn source<F : Format, T>(endpoint: Readable, format: Type<F>) : Iterable<T>
```

The one header covers every form; the forms it takes are:

```text
source<T>(endpoint: Replayable, format: Type<F> = <by extension>) : FiniteSource<T>
source<T>(endpoint: Readable, format: Type<F>) : LiveSource<T>
source<T>(channel: Channel<T>) : Seq<T>
source<S>(id: String) : S   // HQL-0012, unchanged
```

| Expression | Contract | Stands as |
| --- | --- | --- |
| `source(file(p), F)`, `source(http(u), F)` | `FiniteSource<T>` | `Seq<T>` |
| `source(stdin, F)`, `source(websocket(u), F)` | `LiveSource<T>` | `Stream<T>` |
| `source(channel)` | — | `Seq<T>`, what it holds now |
| `source("id")` | as configured | as configured |

- `DataSource`, `FiniteSource` and `LiveSource` MUST be the contracts of
  [HQL-0012](../HQL-0012/README.md), amended in two ways. `DataSource` has no
  `id` field, since a source read from an endpoint has no configured id; a
  configured source's id is a field of its concrete type. A `LiveSource<T>` is
  a `Stream<T>`; a source whose deliveries carry a position, a broker topic,
  is a `LiveSource<Message<T>>`.
- `source` over a replayable endpoint MUST give a `FiniteSource<T>`: every
  document, in order, as a `Seq<T>`. An ordinary JSON file is therefore a
  sequence of one document, `source(file("birds.json"))[0]`.
- `source` over a live endpoint MUST give a `LiveSource<T>`, which stands as a
  `Stream<T>`. Each document MUST be produced as soon as it has been read, so
  `source(stdin, Json) | filter(…)` over a pipe that stays open writes results
  as they arrive.
- `T` MUST be the type the format reads a document as (section "Formats decode
  and encode documents"), unless the expected type gives another (section
  "Typed decoding").
- A configured `json_lines` or `csv` source of HQL-0012 MUST decode its file
  by the same rules, so a file gives the same values whether a program names
  it with `file` or by id. A configured `json` source gives the members of the
  file's one array, which is
  `source(file(p), Json) | of_type(Seq<Data>) | emit`.

### 5. `tap`

`tap` reads an endpoint one document at a time, as a `Stream`.

```hql
effectful fn tap<F : Format, T>(endpoint: Readable, format: Type<F>) : Stream<T>
```

```text
tap<T>(endpoint: Readable, format: Type<F> = <by extension>) : Stream<T>
tap<T>(channel: Channel<T>) : Stream<T>
tap<T>(id: String) : Stream<T>
```

- `tap` MUST give a `Stream<T>` that is not a `Seq<T>`, whatever the endpoint,
  and MUST hold one document at a time. `tap(file("events.jsonl"))` reads a
  file larger than memory; `source` of the same file would hold all of it.
- Over a live endpoint, `tap` and `source` MUST give the same stream:
  `tap(stdin, Json)` is `source(stdin, Json)`.
- `tap("id")` MUST give the configured source's contents as a stream: a
  `StreamSource<T>` or `LiveSource<T>` as it is, and a `FiniteSource<T>` read
  one element at a time.

### 6. `target` and `sink`

A `target` receives one completed value and a `sink` consumes a stream.

```hql
abstract type DataTarget

effectful fn target<T, F : Format>(value: T, endpoint: Writable, format: Type<F>) : Unit
effectful fn sink<T, F : Format>(values: Iterable<T>, endpoint: Writable, format: Type<F>) : Unit
```

```text
target<T>(x: T, endpoint: Writable, format: Type<F> = <by extension>) : Unit
target<T>(x: T, channel: Channel<T>) : Unit
sink<T>(xs: Stream<T>, endpoint: Writable, format: Type<F> = <by extension>) : Unit
sink<T>(xs: Stream<T>, channel: Channel<T>) : Unit
```

- `DataTarget` MUST be the contract of what a program writes to, the mirror of
  `DataSource`. A `Writable` endpoint written in a format is one, and so is a
  channel. A configured target by id, a table or a topic, is a later
  record's, and it MUST be written by the same `target` and `sink`.
- `target` MUST write `x` as one document, formatted as `format(x, F)` would
  format it ([HQL-0017](../HQL-0017/README.md)). A `Seq<Bird>` written by
  `target` in `Json` is one JSON array.
- `target` over a `Stream<T>` that is not a `Seq<T>` MUST be a check error
  naming `sink` and `take(n)`. A stream may not end, so it has no completed
  value to write.
- `sink` MUST write each value of `xs` as one document, as the value arrives.
  A `Seq<Bird>` written by `sink` in `Json` or `JsonLines` is JSON Lines. Because a `Seq` is
  a `Stream`, `sink` accepts one without `emit`.
- `D2` and `Mermaid` describe a whole and MUST be refused by `sink`, naming
  `target`.
- `target` to a file MUST replace it atomically: the document is written
  beside the destination and renamed over it when complete. A reader never
  sees a partial file, and a failed write leaves the old file in place.
- `sink` to a file MUST truncate it before its first document and write each
  document as it arrives. A stream that never ends cannot be renamed into
  place.
- A missing parent directory MUST be an evaluation error; neither creates a
  directory.
- One evaluation MUST NOT replace one file twice, by any combination of
  `target` and `sink` to `file(p)`; the second write is an evaluation error
  naming the path. A read of a file the same evaluation has replaced MUST be
  an evaluation error. A program can read a file and then replace it.
- Writes to `file(p).end` MAY happen any number of times in one evaluation,
  each after the last, as writes to a channel do. One evaluation MUST NOT
  write both `file(p)` and `file(p).end`; the second is an evaluation error
  naming the path.

`target(stdout, F)` and `sink(stdout, F)` differ from the result the host
prints:

- The result of a program is presented by the host in the format `--to`
  names, after evaluation, as today.
- `target(stdout, F)` and `sink(stdout, F)` MUST write while the program runs,
  as part of its evaluation, and MAY be used on a `tee` branch.

### 7. Seq and Stream

What an operation accepts and gives depends on whether the values end. A
`Seq<T>` is a `Stream<T>` that ends, so every rule for a stream holds for a
sequence, and a sequence has more.

| | `Seq<T>` | `Stream<T>` that is not a `Seq<T>` |
| --- | --- | --- |
| `source` gives it | over a `Replayable` endpoint or a channel | over a live endpoint |
| `tap` | reads it one value at a time | reads it as it is |
| `target` | writes one document, a `Json` array | refuses it |
| `sink` | writes one document per element | writes one document per value |
| the result, by `--to` | one whole text | one piece per value, as it arrives |
| `size`, `sort`, `xs[i]`, `contains` | accepted | refused |
| `filter`, `map`, `take`, `drop`, `tee` | accepted | accepted, one value at a time |
| `collect(n)` | gives `List<List<T>>` | gives `Stream<List<T>>`, in batches |

- An operation that needs the end of its input MUST refuse a stream that is
  not a sequence, naming `take(n)` and `collect(n)`. A bounded prefix ends,
  and a batch is finite, so the program says which it means.
- Between them, `source` and `tap` choose how a replayable endpoint is held:
  `source` holds every document, so the result is a `Seq`; `tap` holds one,
  so it is a `Stream`. A live endpoint has no such choice.
- `emit` keeps the meaning of [HQL-0007](../HQL-0007/README.md): it gives the
  members of each sequence a stream carries. It is not needed to turn a
  `Seq` into a `Stream`, because a `Seq` already is one.

### 8. Several outputs

A program writes several outputs by branching with `tee`
([HQL-0010](../HQL-0010/README.md)).

```hql
source(stdin, Json)
| tee archive = sink(file("events.jsonl"))
| filter(e => e.level == "error")
| tee errors = sink(file("errors.jsonl"))
| map(e => e.message)
| sink(stdout, Text)
```

- A branch that ends in `target` or `sink` MUST receive every value the trunk
  passes through the `tee`, in order, before the trunk receives it.
- A failed write on any branch MUST end the evaluation with that error.
- Writes MUST happen in the order the values arrive. A planner MUST NOT
  duplicate, reorder or drop a `target` or `sink`.

### 9. Typed decoding

The expected type of a read chooses what each document decodes as.

```hql
type Sighting {
    species : String
    count   : Int
}

sightings : Stream<Sighting> = source(stdin, Csv)
birds : Seq<Sighting> = source(file("birds.json"))
fn load() : Seq<Sighting> = source(file("birds.jsonl"))
```

- The expected type of a read MUST be the declared type of the binding whose
  whole value it is, or the declared result of the function whose whole body
  it is, written `file(p) | source` or `source(file(p))`. A read inside a
  larger expression has none, and a call names no type argument.
- When the expected type is a `Seq`, `Stream` or other iterable of an element
  `E`, and the format's document type `D` is not `E`, each document MUST
  decode as `E` when a `D` may be an `E`: a declared record read from a
  `Data` tree, or a type that narrows `D`. Otherwise the read keeps its
  default, from the section "Formats decode and encode documents", and the
  binding refuses it. A `Seq<Sighting>` is never read from `Lines`.
- A tree MUST decode as a record by its shape
  ([HQL-0016](../HQL-0016/README.md)), as `match` and `of_type` read it: every
  required field at its type and an optional field without an entry absent.
  An entry the record does not name is refused for a closed record and kept
  beneath an open one, a record narrowing `Data`
  ([HQL-0030](../HQL-0030/README.md)).
- A CSV cell MUST be read as the scalar its field declares, an `Int`, `Float`
  or `Bool` parsed from its text, and an empty cell at an optional field that
  is not a `String` MUST be absent. A `Json`, `JsonLines` or `Yaml` leaf MUST
  keep the type it was written with: `"3"` is not an `Int`.
- A document that does not decode as the expected type MUST be an evaluation
  error naming the endpoint, the document's position, counted from 1, and the
  field's dotted path:

  ```text
  rows.csv: document 2 does not decode as Sighting: field `count`: "many" is not an Int
  ```

  The type is fixed at check time; the data is checked as it is read, so
  a stream gives every document before the one that does not decode.

### 10. Channels

A channel is an endpoint of values, held in memory for one execution scope.

```hql
type Channel<T> : {Readable, Writable} {
    name : String
}

events = Channel<LogEvent>("cell.logs")

event | target(events)           // appends one value
stream | sink(events)            // appends each value as it arrives
source(events)                   // Seq<LogEvent>: what it holds now
tap(events)                      // Stream<LogEvent>: each value as it is written
```

- A channel MUST keep `T`: `source`, `tap`, `target` and `sink` over one take
  no format, and a format argument MUST be a check error.
- Values MUST be read in the order they were written, first in, first out.
- `source(channel)` MUST give the values written so far, in order, as a
  `Seq<T>`, and MUST NOT remove them. Reading a log does not consume it.
- `tap(channel)` MUST give a `Stream<T>` of the values written from then on,
  each as it is written. It ends when the scope does.
- A channel MUST live for its execution scope: a program run, or one cell of
  a document. Within a scope one name MUST be one channel, and constructing a
  name a second time with a different `T` MUST be an evaluation error naming
  both.
- Writing a channel MUST be `Effectful`, so writes happen in program order and
  a `source` after them sees them.
- At the end of a scope the host MAY read every channel, by name, as part of
  what the execution produced beside its result. That is a host protocol and
  adds no value to the language.

### 11. Network endpoints

`import net` adds endpoints that are transports. They are read and written by
`source`, `tap`, `target` and `sink` like any other.

```hql
import net

type Http : {Replayable, Writable} {
    url     : String
    method  : String
    headers : Data
}
type WebSocket : {Readable, Writable} {
    url : String
}
type Socket : {Readable, Writable} {
    host : String
    port : Int
}

fn http(url: String, method: String = "GET", headers: Data = {:}) : Http
fn websocket(url: String) : WebSocket
fn socket(host: String, port: Int) : Socket
```

```hql
birds  = source(http("https://api.example.com/birds"), Json)   // Seq<Data>
alerts = tap(websocket("wss://events.example.com"), Json)      // Stream<Data>

report | target(http("https://api.example.com/report", method = "POST"), Json)
```

- Reading an `Http` MUST send a `GET` and decode the response body. The host
  MUST send it at most once per evaluation, so it is `Replayable` and
  `source` gives a `Seq`.
- Writing an `Http` MUST send one request per document with the document as
  its body, by `method`, and MUST NOT read the response beyond its status. A
  status that is not success MUST be an evaluation error naming the URL and
  the status.
- A `WebSocket` MUST read each message as a document and write each document
  as a message.
- A `Socket` carries bytes, so its documents MUST be separated as their format
  separates them (section "Formats decode and encode documents").
- A program SHOULD NOT write a secret in a header. A request that needs
  credentials is a configured source of HQL-0012, whose secrets stay outside
  the program.

### 12. Logging

`import logging` adds a structured event and a producer of it. A logger writes
to an endpoint by the rules of `sink`; it adds no output of its own.

```hql
import logging

abstract type LogLevel
type Trace : LogLevel
type Debug : LogLevel
type Info  : LogLevel
type Warn  : LogLevel
type Error : LogLevel

type LogEvent {
    level   : LogLevel
    logger  : String
    message : String
    data    : Data
}

type Logger {
    name  : String
    level : LogLevel
}

fn Logger<F : Format, L : LogLevel>(to: Writable, format: Type<F>, *, name: String, level: Type<L> = Info) : Logger
effectful fn info(log: Logger, message: String, data: Data = {:}) : Unit   // and trace, debug, warn, error
```

The format is omitted for a `Channel<LogEvent>`, which takes the events as
values: `Logger(events, name = "birds")`.

```hql
import logging

log = Logger(stderr, Text, name = "birds")

log | info("loading input")
birds = source(stdin, Json) | filter(b => b.species == "owl")
log | info("filter ready", {species: "owl"})
birds | sink(stdout, Json)
```

- The levels MUST be ordered from least to most severe: `Trace`, `Debug`,
  `Info`, `Warn`, `Error`. A level is tested with `e.level | is(Error)`
  ([HQL-0016](../HQL-0016/README.md)), and a threshold is passed as a type.
- `trace`, `debug`, `info`, `warn` and `error` MUST write one `LogEvent` to
  the logger's endpoint when their level is at least the logger's threshold,
  and nothing otherwise. Each MUST be `Effectful` and return `Unit`, so a log
  call never becomes part of a result.
- A logger MUST write as `sink` would: one document per event. To a file in
  `Json` that is JSON Lines; to a channel it is the `LogEvent` value itself.
- A log file is appended to by naming its end,
  `Logger(file("app.log").end, Text, name = "birds")`; a logger on `file(p)`
  replaces the file with this run's events, by the rules of `sink`.
- Formatted as `Text`, a `LogEvent` MUST be one line: the level in capitals,
  the logger's name, the message, then the data as an `Hql` literal unless it
  is the empty `{:}`:

  ```text
  INFO birds filter completed {kept: 2}
  ```

- Formatted as `Json`, a `LogEvent` MUST be one object with the keys `level`,
  `logger`, `message` and `data`, its level written as the level's name.
- A logger's name MUST be a string, dotted by convention, `hql.query`. A
  name's parts carry no meaning in this record.
- Constructing a logger MUST be `Pure`. What a logger writes to MUST be
  checked by the host's policy as a `sink` to that endpoint, so logging grants
  nothing a program could not do with `sink`.
- A host MAY override every logger's threshold; the command line does so with
  `--log-level` (section "The command line").

### 13. Effects and what a host grants

Reading and writing are effects the host performs, and the checker knows every
one before evaluation.

- `source` and `tap` over an endpoint or an id, `target`, `sink` and every log
  call MUST be `Effectful`. Endpoints, channels and loggers are values, and
  constructing one is `Pure`.
- A function whose body uses an `Effectful` operation MUST be `Effectful`, so
  the checker can report every endpoint a program may use.
- A binding of `source` or `tap` over a byte endpoint MUST be read once per
  evaluation, however many times it is used.
- The host MUST check each use against its policy before evaluation, and
  refuse a use it does not grant with a check error naming the endpoint.

| Host | Reads | Writes |
| --- | --- | --- |
| `hql eval`, `hql run`, `hql repl` | any endpoint | any endpoint |
| `hql render` | files under the document's directory, channels | channels |
| `hql check` | none | none |

- Under `hql render`, a `file` read whose path resolves outside the document's
  directory, by `..` or by a symbolic link, MUST be an evaluation error naming
  the path.
- Under `hql render`, every write to an endpoint that is not a channel MUST be
  refused at check time, including `stdout` and `stderr`, and every use of a
  network endpoint MUST be refused at check time. A cell's result is what it
  contributes to the document.
- `hql check` MUST perform no read and no write but reading the program.

### 14. Cells and their logs

A cell of a document ([HQL-0009](../HQL-0009/README.md)) has one result and
may produce logs beside it.

```hql
import logging

log = Logger(cell.logs, name = "bird-report")

log | info("query started")
birds = cards | filter(c => c.metadata.category == "bird")
log | debug("cards selected", {count: birds | size})
birds | map(c => c.title)
```

- `hql render` MUST bind `cell` for each cell, with a field
  `logs : Channel<LogEvent>` that is the cell's own channel.
- The cell's result MUST be its value, unchanged by anything it logged.
- The host MUST NOT write a channel's contents into the document's saved
  outputs. Logs are transient: they depend on the run and would leave stale
  text in the document. A host MAY show them beside the result.
- How a cell asks for its logs to be shown, or saved, belongs to the cell
  syntax of HQL-0009, not to this record.

### 15. The command line

The command line presents a result as `jq` does, and names no input or output
of its own.

- `--to json` MUST present each document of a result pretty-printed, one
  after another, as `jq .` does. Over a `Seq` that is the whole sequence as
  one array; over a stream that is not a sequence, each value as it arrives.
  Concatenated JSON values are what `source(…, Json)` reads, so the output of
  one `hql` is the input of the next.
- `--compact` MUST present each document of a `Json` result on one line, as
  `jq -c` does. A stream is then JSON Lines.
- `sink(e, Json)` MUST write one document per line, whatever the flags. A
  program writing a file says what the file is; `--compact` is about the
  terminal.
- `--log-level <level>` MUST set every logger's threshold, overriding the one
  it was constructed with; `<level>` is `trace`, `debug`, `info`, `warn` or
  `error`.
- A first argument that is not a command name MUST be read as
  `hql eval <PROGRAM>`. A program whose whole text is a command name is
  written with `eval`.
- A result of type `Unit` MUST be written as nothing, so a program that ends
  in `target` or `sink` prints no value.
- The command line MUST NOT name a program's input or its outputs. Input is
  `source` or `tap` in the program; output is `target` or `sink` in the
  program, or the result, presented by `--to`.

Together:

```sh
hql 'source(stdin, Json)' --to json < birds.json          # jq .
hql 'source(stdin, Json)' --to json --compact < log.jsonl  # jq -c .
hql 'source(stdin, Json)' --to yaml < birds.json          # JSON in, YAML out
hql 'source(stdin, Json) | map(b => b.sightings | size)' < birds.json
tail -f events.jsonl | hql 'source(stdin, Json) | filter(e => e.level == "error")'
hql 'source(file("birds.json")) | map(b => b.sightings) | target(file("sightings.csv"))'
hql 'tap(file("events.jsonl")) | filter(e => e.ok) | sink(file("ok.jsonl"))'
hql run --log-level debug birds.hql < birds.jsonl 2> run.log
```

## Backwards Compatibility

- `Endpoint`, `Readable`, `Replayable`, `Writable`, `Stdin`, `Stdout`,
  `Stderr`, `File`, `FileEnd`, `Channel`, `DataSource`, `FiniteSource`, `LiveSource`,
  `DataTarget`, `stdin`, `stdout`, `stderr`, `file`, `tap`, `target` and
  `sink` are new names in the prelude. A program binding one of them shadows
  it.
- `Http`, `WebSocket`, `Socket`, `http`, `websocket` and `socket` exist only
  after `import net`; `LogLevel`, its five levels, `LogEvent`, `Logger`,
  `trace`, `debug`, `info`, `warn` and `error` only after `import logging`.
  No existing program imports either.
- [HQL-0012](../HQL-0012/README.md)'s `DataSource` loses its `id` field and its
  `LiveSource<T>` becomes a `Stream<T>`, as the section "`source`" states.
  HQL-0012 is drafted and not implemented, so no program changes.
- `source(id)` keeps HQL-0012's meaning; `source(endpoint, Format)` and
  `source(channel)` are new overloads.
- [HQL-0017](../HQL-0017/README.md) reserves `--from <format>` for standard
  input. This record replaces the reservation: the format of standard input is
  the format argument of `source(stdin, …)`, and `--from` is not added.
- [HQL-0017](../HQL-0017/README.md) writes each piece of a `Json` stream on one
  line. The host now pretty-prints each piece unless `--compact` is given;
  `format` and `sink` keep one value per line.
- `hql run -` keeps reading the program from standard input. Such a program
  cannot also read `stdin` as data, and `source(stdin, …)` in it MUST be a
  check error.
- A first argument that is not a command name was a usage error and is now a
  program. Every existing invocation keeps its meaning.
- A `Unit` result printed `()` and now prints nothing.
- The core `debug` step, which captured a value into the run's report queue,
  is removed; `debug` is the logging operation of `import logging`, and a
  program reports on itself through a `Logger`.
- The standard library's module names now follow folders, and
  `tests/declarations.rs` walks `std/` recursively.

## Security Considerations

- **A document writes only its own channels.** `hql render` refuses at check
  time every write to a file, `stdout`, `stderr` or the network, so rendering
  a document someone else wrote changes no file and sends nothing. Logging
  through a logger is checked as the `sink` it is, so it opens no way around
  this.
- **A document reads only beside itself.** Under `hql render`, file reads
  outside the document's directory are refused, including through `..` and
  symbolic links, and the network is refused.
- **Network use is visible and checked.** `net` must be imported, and every
  network endpoint used is known before evaluation; the host refuses what it
  does not grant.
- **Secrets stay out of programs.** A header is ordinary data in the program's
  text; credentials belong to a configured source.
- **Effects are known before evaluation.** Every endpoint use is `Effectful`
  and is checked against the host's policy before anything runs.
- **Checking is offline.** `hql check` touches no file and no network but the
  program, as HQL-0012 requires of sources.
- **Atomic replacement.** An interrupted `target` leaves the old file, never a
  truncated one.
- **Resource bounds.** HQL-0012's `max_bytes` MUST bound `source` over a
  replayable endpoint, which holds every document, including an HTTP
  response. `tap` holds one document at a time and is bounded by its consumer.
  A channel holds what its scope wrote; the host MUST bound it and end the
  scope with an error naming the channel when it is exceeded.

## Reference Implementation

Built:

- `std/io/mod.hql`, `std/io/net/mod.hql`, `std/io/logging/mod.hql` — the types and
  the `effectful fn` and `fn` headers of every operation, bound to the
  extensions that implement them; `std/format/mod.hql` — the formats and their
  `Decoder` and `Encoder` headers
- `src/library.rs` and `tests/declarations.rs` — modules named by folder, a
  folder's `mod.hql` by the folder
- `src/io.rs` — the I/O itself: endpoints, the decoders and encoders by
  format, file endpoints relative to the source unit, atomic and streaming
  writes, following a file's end, channels, and the host policy
- `src/extensions/io.rs` — the `io` extension in the prelude: `file`,
  `source`, `tap`, `target` and `sink`, endpoint capabilities, the format a
  literal extension implies, and the decoding of each document as the element
  type a read is expected to give
- `src/checker.rs` — the expected type of a read, `Effectful` propagated
  through functions
- `src/types/value.rs` — `Value::decoded`, a tree read as a declared record,
  naming the field that does not read; `src/traversal.rs` and
  `src/evaluation.rs` — a live read decoded one document at a time
- `src/extensions/net.rs` and `src/net.rs` — the `net` extension: HTTP,
  WebSocket and TCP endpoints
- `src/extensions/logging.rs` — the `logging` extension: `Logger`, the five
  operations, and the `Text` and `Json` forms of a `LogEvent`
- `src/document.rs` and `src/render.rs` — the `cell` binding and its logs
  channel, kept out of saved outputs
- `src/cli.rs` — the default `eval`, `--compact`, `--log-level`,
  pretty-printed `Json` pieces, and `Unit` written as nothing
- `tests/io.rs`, `tests/net.rs`, `tests/logging.rs`, and the
  [I/O examples](../../../examples/io/README.md)
- `doc/models/behavior/cli.md` and `doc/wiki/hql/extensions.hmd` — `io`,
  `net` and `logging` in the command table and the provider table

Remaining, each waiting for [HQL-0012](../HQL-0012/README.md), which is
drafted:

- `source("id")` and `tap("id")`, a configured source read by id, and a
  configured `json_lines`, `csv` or `json` source decoded by this record's
  rules
- HQL-0012's `max_bytes` bounding `source` over a replayable endpoint

## Test Plan

Unit tests MUST include:

- each row of the format table: `source` over a file fixture with one document
  and with several, and `tap` over the same
- a JSON array read as one document, and spread by `of_type(Seq<Data>) | emit`
- `source(file(p))` with a non-literal `p` refused, naming the format
  argument; an unknown extension refused the same way
- `source(stdout, Json)` and `42 | target(stdin, Json)` refused, naming the
  capability
- `target` over a stream that is not a `Seq` refused, naming `sink` and
  `take(n)`; `sink` over a `Seq` accepted
- `size` over `source(stdin, Json)` refused, naming `take(n)` and `collect(n)`;
  over `source(file(p), Json)` accepted
- `Seq<Data>` written by `target` in `Json` is one array, and by `sink` is
  JSON Lines
- a CSV read into `Stream<Sighting>`, and a row whose `count` is not an `Int`
  refused, naming the row and the field
- two replacements of one file refused; a read after a replacement refused; a
  replacement after a read allowed
- two appends to `file(p).end` in one evaluation both kept, in order; a
  replacement and an append of one path in one evaluation refused
- `source(file(p).end, Json)` refused, naming `tap`
- a channel: values read in write order; `source` twice gives the same values;
  a format argument refused; one name with two element types refused
- `http` used without `import net` refused; `file("https://…")` refused,
  naming `http`
- a logger below its threshold writes nothing; a `LogEvent` in `Text` and in
  `Json` has the forms the section "Logging" states
- a function calling `source` or `info` reported as `Effectful`
- in the `render` context: a channel write accepted; a `sink` to a file, a
  logger on `stderr` and any `http` refused at check time
- a URL path refused

Integration tests MUST include:

- `hql 'source(stdin, Json)' --to json < x.json` writes the document
  pretty-printed, `--compact` on one line, and `--to yaml` in YAML
- the output of `hql 'source(stdin, Json)' --to json` read again by
  `source(stdin, Json)` gives the same values
- `source(stdin, Json) | filter(…)` over a pipe that stays open writes the
  first result before the input ends
- `hql '1 + 2'` with an open, silent standard input exits at once
- `source(stdin, Json) | sink(file("out.jsonl"))` reproduces its JSON Lines
  input byte for byte
- three `tee` branches ending in `sink` each write their file, and a failure
  in one ends the evaluation
- an interrupted `target` leaves the previous file intact
- `sink(file(p).end, Json)` run twice leaves both runs' documents in `p`
- `tap(file(p).end, Text)` gives a line appended by another process after the
  tap began, and none written before it
- a relative `file` in `hql run sub/q.hql` resolves against `sub/`
- a program logging to `stderr` writes its result to stdout and its log lines
  to stderr, and `--log-level warn` drops its `info` lines
- `source` over `http` against a local test server, and a failed `POST`
  status reported with the URL
- `hql render` of a cell that logs to `cell.logs`: the saved output holds the
  result only
- `hql render` of a document whose cell reads `../outside.json` refused

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Open Questions

None.

## Changelog

- 2026-10-19: drafted
- 2026-10-22: accepted, implemented in part. Typed decoding: the declared
  type of a binding or a function result decodes each document of a read as
  a declared record, a CSV cell parsed as its field's scalar. The
  specification states `std/io/mod.hql`, `JsonLines` and `Lines` as formats,
  and the `effectful fn` headers; configured sources by id and `max_bytes`
  wait for HQL-0012
  ([issue 0037](../../issues/0037-io-typed-decoding.md))
- 2026-10-22: `source("id")` reads a configured source and `tap("id")` a
  finite one one element at a time, a file of documents as `tap` reads the
  file; `max_bytes` bounds a configured source
  ([issue 0040](../../issues/0040-configured-data-sources.md))
