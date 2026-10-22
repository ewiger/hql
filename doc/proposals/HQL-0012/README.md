# HQL-0012: Data sources

- **Status**: accepted, implemented in part
- **Created**: 2026-10-18
- **Source**: [HQL is not an HMD query language](../../wiki/hmd-integration.hmd)

## Abstract

A **data source** is a named endpoint outside the program whose contents HQL
queries: a vault of documents, a relational database, a message log, a queue,
an HTTP API. A program names one by its **id** and uses the bound value
directly. A source is a value of the type of its contents, so
`warehouse = source("warehouse")` followed by `warehouse | filter(…)` is an
ordinary pipeline, with no verb between the source and its first step. The core
declares `DataSource` and the contracts that fix what a source's contents are:
`FiniteSource<T>`, a `Seq<T>` in the source's stated order; `KeyedSource<K, T>`,
which adds lookup with `get`; `StreamSource<T>`, a replayable
`Stream<Message<T>>`; `LiveSource<T>`, a stream with no replay; `Catalog<S>`, an
endpoint that contains sources; and `VaultSource`, documents read as cards.
Every concrete source type — `Vault`, `PostgresTable`, `KafkaTopic` — is
provided by an extension. An id is resolved at check time against the
configuration of the run context, which holds every connection detail and
secret. A portion of a source is taken with the operations every sequence and
stream already has — `take`, `filter`, slicing, `collect` and `emit` — and how
the host fetches it is not part of the language. The vault a program runs
against is the source `vault`, and `cards` is `vault.cards`, its cards in the
order of a sorted walk.

## Companion notes

- [Names and where they live](../../wiki/hql/naming.hmd) — the casing rules the
  source types, ids and operations below follow.

## Motivation

- [HQL is not an HMD query language](../../wiki/hmd-integration.hmd) states
  that documents, Postgres, graph stores and object buckets are peer sources,
  and the [index page](../../index.md) names message queues such as Kafka and
  REST APIs beside them. None of them has a type, so there is nothing for a
  program to hold, check or pass to a step.
- The vault is the only source HQL has, and it is special: no source contract
  says what it is, and no second vault can sit beside it. The
  [queries model](../../models/behavior/queries.md) tries to make it ordinary by
  writing a function over a `vault` value. A source is not a function a program
  writes; it is a value the run context supplies, and the operations over it
  are the same for every source of a kind.
- A query that names a connection string is a query that cannot move between
  machines, cannot be shared without leaking a secret, and cannot be checked
  without a network. The id is the indirection that keeps a query portable.
- [HQL-0004](../HQL-0004/README.md) and [HQL-0006](../HQL-0006/README.md) both
  assume sources — a backend that pushes `filter` into SQL, a stream that never
  ends — without saying what a source is.

## Goals

- one abstract type, with contracts that say what a source's contents are, in
  what order, and whether using them is pure
- static typing: the type of `source("id")` is known at check time, offline
- a catalog of concrete source kinds broad enough that a new integration is an
  extension supplying a type, not a change to the core
- one place for connection details and secrets, outside every query

## Non-goals

- **No writes.** Inserting a row, publishing a message or acknowledging one is
  an effect. Effectful steps over sources are a later proposal; this one reads.
- **No native query strings in the core.** A `filter` over a table is
  HQL, and pushing it into SQL is the planner's work under HQL-0004. An
  extension MAY offer an escape hatch, which the section "Native queries" bounds.
- **No implementation of any network source.** The kinds read from local
  files, `vault`, `csv`, `json`, `json_lines`, `yaml` and `sqlite`, are
  implemented; every network kind in the section "Source kinds" is design
  direction.
- **No dynamic sources.** A program cannot create a source at evaluation time
  from a computed string.

## Streaming, batching and portion-wise access

A source is rarely taken whole. A table can be larger than memory, an API
returns its collection a page at a time, and a topic never ends. A program needs
to take a source a portion at a time, and the question this section settles is
how that portion is written.

### Two kinds of source

A **finite source** has contents that exist as a whole at a snapshot: a vault,
a table, a file, a REST collection. Any part of it can be addressed by position
or by key, and asking twice gives the same answer within one evaluation.

A **stream source** has contents that arrive over time: a Kafka topic, a NATS
subject, a work queue. A replayable log can be addressed by position; live
delivery cannot be addressed at all, only waited on.

The two take portions differently. A finite source is cut — this many, from
here. A stream is bounded — this many, or for this long, whichever comes first.

### Ways to take a portion of a finite source

- **Offset and limit.** "Skip 100, take 50." Simple and positional, and the
  form SQL, most ORMs and many APIs expose. Its cost grows with the offset, and
  without a snapshot a concurrent insert shifts every later page by one.
- **Keyset, or seek.** "After key `k`, take 50." Each page starts where the
  last one ended in the source's total order, so cost is constant and an insert
  shifts nothing. It cannot jump to page 40 without walking there, and it needs
  a total order on a key — which the section "Using a source" already requires of every finite
  source.
- **Opaque cursors.** A continuation token the source mints and the client
  hands back, as most REST and GraphQL APIs do. It is the source's keyset or
  offset, hidden. A program cannot construct one, only pass one along.
- **Prefix.** "The first 50." The special case every other form starts with.
- **Slicing.** `xs[100:150]`, as in Python: offset and limit written as an
  operation on the value rather than as parameters of a request. It applies to
  any sequence, not only to a source.

### Ways to bound a stream

- **Position range.** "From offset 1000 to offset 2000." Deterministic on a
  replayable log, and the stream equivalent of a slice. Live delivery has no
  positions to name.
- **Count.** "The next 100 messages." Deterministic on a replayable log from a
  fixed position; on live delivery it waits until 100 arrive.
- **Time.** "Whatever arrives in 5 seconds." The only bound that guarantees an
  answer on a quiet stream, and the only one that depends on the clock rather
  than on the data.
- **Windows.** Count and time together, repeated: "batches of up to 100, each
  closed after at most 5 seconds", with the pipeline continuing over the
  batches. This is the shape of most stream processing, and it does not end the
  stream. In HQL it is `collect`, below.
- **Polling.** A loop that asks the source for whatever is ready, then asks
  again. It is how client libraries are built, but a loop with a timeout per
  request is host machinery; a query states what it wants, not how often to ask.

### What the interface should be

The aim is one interface flexible enough to cover every row above without a
separate API per source kind.

- **A source is used directly.** It is bound once and then piped, with no verb
  between the binding and the first step:

  ```hql
  warehouse = source("warehouse")
  warehouse | postgres.sql("select species, count(*) from sightings group by 1")
  ```

  The same holds for every kind. A finite source stands where its contents
  would, and a stream source stands where its stream would:

  ```hql
  notes     = source("field_notes")
  sightings = source("sightings")

  notes | filter(c => c.metadata.genre == "field") | take(10)
  sightings | of_type(Sighting) | collect(100, for = 5s) | map(…) | emit
  ```

  Typing and matching apply as they do to any value: `of_type(Sighting)` narrows
  `Data` elements to a declared type, and `match` branches on them.
- **Portions are ordinary operations.** The program writes the portion with
  operations that mean the same on every sequence — a slice, a `take`, a
  `filter` on a key, a `collect` window — and the source carries them out in its
  own terms: `OFFSET` and `LIMIT` for SQL, a seek for a keyset, a cursor walk
  for a REST API, a consumer position for Kafka. That is pushdown under
  [HQL-0004](../HQL-0004/README.md), and it is held to the same rule: the result
  is the one the operation would give over the whole contents.
- **Keyset paging needs no new syntax.** "After `k`, take 50" is
  `xs | filter(r => r.id > k) | take(50)` over a source ordered by `id`, and a
  source with an index on `id` turns it into a seek. A cursor type is needed
  only where a source offers nothing but opaque tokens, and then it stays
  inside the source.
- **Paging is the source's business.** A program never sees a page boundary
  unless it asks for one with a slice. How many requests a REST walk takes, or
  how many rows one round trip fetches, is configuration of the run context,
  not program text.

### Batching is `collect`, and `emit` undoes it

A slice answers "which portion". Batching answers a different question: "all of
it, a portion at a time". The operation for that is `collect`, drafted in
[HQL-0007](../HQL-0007/README.md), and its inverse is `emit`.

```text
collect<T>(xs: Stream<T>, n: Int) : Stream<List<T>>
collect<T>(xs: Seq<T>,    n: Int) : List<List<T>>
emit<T>(xs: Stream<Seq<T>>)       : Stream<T>
emit<T>(xs: Seq<Seq<T>>)          : List<T>

xs | collect(n) | emit  =  xs
```

`collect(n)` groups consecutive values into batches of `n`, the last possibly
fewer. It does not end the pipeline: the batches flow on, and every step after
it sees one `List<T>` at a time. `emit` puts the members of each batch back into
the pipeline, in order, so a pipeline can batch for one stretch and continue
value by value after it.

The same pair serves both kinds of source.

- **Over a finite source, batching is paging done by the program.**
  `rows | collect(500)` is the table in pages of 500, in its stated order, and
  a step that works best on many rows at once — scoring them together, sending
  them in one request — runs once per page:

  ```hql
  rows = source("sightings_table")
  rows | collect(500) | map(page => score_together(page)) | emit
  // score_together stands for any step over a whole page
  ```

  The source sees the batch size as a hint. A SQL source fetches 500 rows per
  round trip, a REST source asks for pages of 500 where the API allows it. The
  batches the program sees are positional and identical either way, because
  `collect` fixes them by position, not by fetch.
- **Over a stream, batching is the window.** `collect(100)` closes a batch at
  100 messages. HQL-0007 reserves `collect(for = 5s)` and
  `collect(100, for = 5s)`: a batch closes at whichever bound comes first. That
  is the count-and-timeout bound a stream needs, written once as a step rather
  than as parameters of every source:

  ```hql
  sightings = source("sightings")
  sightings
  | collect(100, for = 5s)
  | map(batch => batch | sort(by = m => m.time))
  | emit
  ```

  A batch closed by time holds whatever arrived before it closed. Whether a
  window in which nothing arrived yields an empty batch or none is HQL-0007's
  to state when time windows land.

`collect` replaces the polling loop. A loop that asks a client library for
"up to 100, waiting at most 5 seconds" is exactly one `collect(100, for = 5s)`
batch, and the loop around it is the stream continuing. The program states the
window; the source decides how to fill it.

Slicing and batching compose. `rows[0:10000] | collect(500)` is the first ten
thousand rows in twenty pages; `sightings | take(1000) | collect(100)` is the
first thousand messages in ten batches.

### What exists and what is missing

Most of this vocabulary is already in the core or drafted:

| Operation | Takes a portion by | Over | Where |
| --- | --- | --- | --- |
| `take(n)` | prefix, count | finite and stream | core |
| `filter(p)` | key range, predicate | finite and stream | core |
| `sort(by = …)` | the order a slice is taken in | finite | core |
| `xs[i]` | one position | finite | core |
| `of_type(T)`, `match` | type | finite and stream | [HQL-0016](../HQL-0016/README.md), designed |
| `collect(n)` | count window | stream | [HQL-0007](../HQL-0007/README.md) |
| `collect(n, for = 5s)` | count and time window | stream | HQL-0007, reserved |
| `emit` | ends a window | stream | HQL-0007 |
| `List` | materializes | finite | HQL-0007, HQL-0016 |

Two pieces are missing. Slicing, `xs[a:b]`, extends the indexing that `xs[i]`
already is; its handling of negative bounds and a step is for its own record.
A duration literal, `5s`, is what `collect(…, for = 5s)` and any time bound
wait on.

## Specification

### 1. Terms

Each term below has one meaning.

- **data source**: a named endpoint outside the program whose values HQL reads.
- **source id**: the name a source has in configuration. A program refers to a
  source by its id and by nothing else.
- **source kind**: what kind of endpoint a source is — `vault`, `postgres`,
  `kafka`. A kind is provided by exactly one extension, which also provides its
  HQL type.
- **source binding**: an ordinary binding whose value is a source,
  `notes = source("field_notes")`. The binding's name is the program's; the id
  is the configuration's.
- **anchor**: a binding a query starts from. `cards` is the anchor over the
  default vault.

### 2. The abstract types

The core declares the contracts in the `io` module, `std/io/mod.hql`. Each source
contract is also the type of the source's contents, so a source stands wherever
its contents would.

```hql
// A named endpoint outside the program.
abstract type DataSource {
    id : String     // its id in the run context's configuration
}

// Finite contents, in an order the source states.
abstract type FiniteSource<T> : {DataSource, Seq<T>}

// Finite contents addressable by key: get(s, key) gives Option<T>.
abstract type KeyedSource<K, T> : FiniteSource<T>

// A log: contents in order, replayable within its retention.
abstract type StreamSource<T> : {DataSource, Stream<Message<T>>}

// Delivery with no replay: what has passed is gone.
abstract type LiveSource<T> : {DataSource, Stream<Message<T>>}

// An endpoint that contains sources: a database of tables, a cluster of topics.
abstract type Catalog<S> : DataSource

// A vault: its cards, knowledge and graph as fields (HQL-0020). What the
// documents sit on — a filesystem, a git revision, an HTTP server — is the
// concrete type's.
abstract type VaultSource : DataSource

// One element of a stream, with where and when it sits in it.
type Message<T> {
    value    : T
    position : Position
    key?     : Data
    time?    : Date
    headers  : Data
}

// A message's place in its log, in the log's order.
type Position : Orderable
```

- Binding a source fetches nothing. Its contents are produced as far as a
  consumer demands, as for any lazy `Seq` or `Stream` in the
  [queries model](../../models/behavior/queries.md).
- A concrete source type MUST narrow exactly one of `FiniteSource`,
  `StreamSource`, `LiveSource`, `Catalog` or `VaultSource`. The contract fixes what the
  source's contents are, so it is not optional.
- `VaultSource` is the one domain contract the core declares, because `cards`
  is defined over it (the section "The default source and the `cards` anchor"). It is abstract, so it has no runtime
  constructor: every vault value is of a concrete type an extension provides,
  and a program written against `VaultSource` runs unchanged on each of them.
  It is a contract in the sense of an abstract type, not a trait.
- `Message<T>` carries `position` and `time` because a program filters on them
  to choose where a stream starts (the section "Using a source").
- `StreamSource` and `LiveSource` depend on `Stream<T>`, which
  [HQL-0006](../HQL-0006/README.md) adds to the lattice.

### 3. Naming a source

A program reaches a source through `source`, resolved at check time.

```text
source(id: String) : S        where S is the type the configuration names for id
```

- The argument MUST be a string literal. A computed id would make the result
  type unknowable before evaluation, so a non-literal argument is a type error.
- An id MUST match `[a-z][a-z0-9_]*`, the `snake_case` of
  [naming](../../wiki/hql/naming.hmd), so that `field_notes = source("field_notes")`
  reads as one name.
- An id with no entry in configuration is a name error that names the id and,
  when one is within edit distance 2, the nearest configured id.
- A string that is not a valid id — `source("postgres://db/notes")` — MUST be
  refused with a diagnostic saying that connection details belong in
  configuration.
- The checker MUST NOT open a connection, resolve a host or read a secret.
  Typing a source needs its kind and declared element type, both of which are
  in configuration.

### 4. Configuration

Sources are declared by the context the binary runs in, never by a program.

```toml
[sources.field_notes]
kind = "vault"
root = "notes"                       # relative to the vault containing hql.toml

[sources.warehouse]
kind         = "postgres"
host         = "db.internal"
database     = "birds"
user         = "reader"
password_env = "WAREHOUSE_PASSWORD"  # the variable's NAME, as in HQL-0002

[sources.sightings]
kind     = "kafka"
brokers  = ["kafka-1:9092", "kafka-2:9092"]
topic    = "sightings"
element  = "Data"
start    = "earliest"
```

- Sources MUST be declared under `[sources.<id>]` in `hql.toml`. A user-level
  configuration file and an environment override are the run context's
  business and MAY add or replace entries, with the vault's file as the base.
- A secret MUST be given as the name of an environment variable, in a key ending
  `_env`, following [HQL-0002](../HQL-0002/README.md). A configuration loader
  MUST refuse a key named `password`, `token`, `secret` or `api_key` holding a
  literal value.
- `kind` MUST name a kind an imported extension provides; an unknown kind is an
  error at configuration load, naming the extensions that provide kinds.
- `element` MAY name the element type, a type the core or an extension provides.
  It defaults to the kind's own element type, which is `Data` for every kind
  that reads untyped records. A program narrows further with `of_type` or
  `match`, as [type interpretation](../../wiki/hql/type-interpretation.hmd)
  describes.
- A finite kind with no intrinsic order MUST be configured with one, as
  `order = ["id"]`; a finite source whose order is unstated is an error at
  configuration load. The section "Using a source" states why.
- How a stream is treated is configuration, never program text. A
  `StreamSource` MUST be configured with `start`: `"earliest"`, the oldest
  retained message; `"latest"`, the newest; a position in the log's own terms;
  or an RFC 3339 time. `"latest"` is resolved once, when the evaluation begins,
  so every use of the source in that evaluation starts at the same message. A
  `LiveSource` has no `start`, because it has nothing to start from but now.
  The same program runs over the full history or only the newest messages by a
  change of configuration alone; within one configuration, a program can only
  narrow where it starts, by the filter of the section "Using a source".

### 5. Using a source

A source is used as the value of its contents. These rules state what a program
observes; how the host fetches is not among them.

**Order.** The contents of a finite source MUST be in the order the source
states, and that order MUST be total. A vault's cards come in the order of a
sorted walk ([HQL-0020](../HQL-0020/README.md)); a table by its configured
order or its primary key. Without this, `table | take(5)`
answers differently on two machines, which is the determinism requirement of
the [queries model](../../models/behavior/queries.md) applied to every source
rather than to the vault alone.

**Stability.** Every use of one finite source within one evaluation MUST observe
the same contents. Two pipelines over `warehouse` in one program see one table,
even while it is being written to.

**Where a stream starts.** A stream source's contents begin where its
configuration's `start` says (the section "Configuration"). A program narrows that with an ordinary filter on `position` or `time`,
`sightings | filter(m => m.time >= d)`, and a source MAY carry such a filter
out as a seek. The messages are the same either way.

**Portions.** A portion is taken with operations that mean the same over every
sequence or stream — `take`, `filter`, `sort`, slicing, `collect` and `emit` —
as the section on streaming, batching and portion-wise access lays out. A
source MAY carry any of them out natively, under
[HQL-0004](../HQL-0004/README.md)'s rule that the result is the one the
operation gives over the whole contents.

**Lookup.** `get(s, key)` over a `KeyedSource<K, T>` is the core `get`, and
returns `Option<T>`.

**Purity.** A finite source and a stream source are `Pure`, in the sense
[pipes](../../wiki/hql/types/pipes.hmd) gives the word: their contents are
fixed within an evaluation, or by position. A live source is `Effectful`: its
contents depend on when they were taken, and a planner MUST NOT duplicate,
replay or reorder its use.

**No consumption.** Using a source MUST NOT acknowledge, commit or remove
anything another reader of that source would observe. A kind that cannot avoid
it — a work queue whose receive hides a message from other consumers — MUST be
typed `LiveSource`, so that its use is `Effectful`.

**Membership.** `member(c, name)` names a source inside a catalog by a string
the catalog defines — a table, a topic, a subject. Its existence is not checked
offline. A missing member is an evaluation error naming the catalog's id and
the member, reported through the
[structured log](../../wiki/hql/error-reporting.hmd). Its type is the catalog's
member type `S`.

### 6. The default source and the `cards` anchor

The vault a program runs against is a source like any other.

- When the binary runs with a vault, the host MUST bind `vault` to that vault,
  with the id `vault`. Its static type is `VaultSource`; its value is of the
  concrete type the run context supplies, `Vault` for a vault on the
  filesystem. Without one, `vault` is unbound, and using it is the ordinary
  unbound-name error.
- `cards` is `vault.cards`, a field of the vault and not the vault itself, as
  [HQL-0020](../HQL-0020/README.md) specifies. `cards` is an anchor: an
  ordinary binding, and nothing about it is a keyword. Because `vault` is
  typed by the contract, `cards` means the same on a filesystem vault, a pinned
  git revision and a remote vault.
- `[sources.vault]` MUST NOT be declared; the id is the run context's.

This replaces the iterating function the [queries model](../../models/behavior/queries.md)
sketches for `cards`. Iterating functions remain a language feature in their own
right, in [issue 0010](../../issues/0010-yield-and-bind.md); `cards` does not
depend on them.

### 7. Native queries

An extension MAY expose a step that sends text in the source's own language.

```hql
warehouse = source("warehouse")
warehouse | postgres.sql("select species, count(*) from sightings group by 1")
```

- Such a step MUST be qualified by its extension and MUST return `Seq<Data>`,
  since the checker cannot type the text.
- It MUST run read-only: inside a read-only transaction, or refused when the
  protocol has none.
- It is outside HQL-0004's observational-equivalence guarantee: the planner MUST
  treat it as an opaque finite source and MUST NOT push anything into it.

### 8. Source kinds

Each kind below is provided by an extension, which provides the HQL type and the
reader. `vault`, `csv`, `json`, `json_lines`, `yaml` and `sqlite` with one
`table` are implemented, by the binary's `io` extension; every other row is
design direction.

**Documents.**

| Kind | HQL type | Contract | Element | Order |
| --- | --- | --- | --- | --- |
| `vault` | `Vault` | `VaultSource` | `Card` | sorted walk, from a filesystem |
| `git_vault` | `GitVault` | `VaultSource` | `Card` | sorted walk, at a pinned revision |
| `remote_vault` | `RemoteVault` | `VaultSource` | `Card` | sorted walk, over HTTP or WebDAV |
| `bucket` | `ObjectBucket` | `KeyedSource<String, Data>` | object metadata and body | object key |

A `git_vault` pins a revision in configuration, which makes it the one document
source whose snapshot is the same on every machine and every day.

**Files.**

| Kind | HQL type | Contract | Element | Order |
| --- | --- | --- | --- | --- |
| `csv` | `CsvFile` | `FiniteSource<Data>` | a row keyed by header | file order |
| `json_lines` | `JsonLinesFile` | `FiniteSource<Data>` | one line | file order |
| `json` | `JsonFile` | `FiniteSource<Data>` | one array element | array order |
| `yaml` | `YamlFile` | `FiniteSource<Data>` | one document | file order |
| `parquet` | `ParquetFile` | `FiniteSource<Data>` | a row | file order |

**Relational and analytical databases.**

| Kind | HQL type | Contract | Element | Order |
| --- | --- | --- | --- | --- |
| `postgres` | `PostgresDatabase` | `Catalog<PostgresTable>` | — | — |
| — | `PostgresTable` | `KeyedSource<Data, Data>` | a row | configured, else primary key |
| `sqlite` | `SqliteDatabase` | `Catalog<SqliteTable>` | — | — |
| `mysql` | `MysqlDatabase` | `Catalog<MysqlTable>` | — | — |
| `duckdb` | `DuckdbDatabase` | `Catalog<DuckdbTable>` | — | — |

A configured source MAY name one table directly, `table = "sightings"`, and is
then that table's type rather than the catalog's.

**Document, key-value and search stores.**

| Kind | HQL type | Contract | Element | Order |
| --- | --- | --- | --- | --- |
| `mongodb` | `MongoCollection` | `KeyedSource<Data, Data>` | a document | `_id` |
| `redis` | `RedisKeyspace` | `KeyedSource<String, Data>` | a value | key |
| `nats_kv` | `NatsKvBucket` | `KeyedSource<String, Data>` | the latest value of a key | key |
| `nats_objects` | `NatsObjectStore` | `KeyedSource<String, Data>` | object metadata and body | object name |
| `search` | `SearchIndex` | `KeyedSource<String, Data>` | a document | document id |

A search index is read as documents. Ranking against it is retrieval, which
stays with the steps of [HQL-0002](../HQL-0002/README.md) and returns `Hit`s.

**Logs and streams.**

| Kind | HQL type | Contract | Element | Position |
| --- | --- | --- | --- | --- |
| `kafka` | `KafkaTopic` | `StreamSource<Data>` | a record | partition and offset |
| `nats_stream` | `NatsStream` | `StreamSource<Data>` | a JetStream message | stream sequence |
| `redis_stream` | `RedisStream` | `StreamSource<Data>` | an entry | entry id |
| `kinesis` | `KinesisStream` | `StreamSource<Data>` | a record | shard and sequence |
| `postgres_changes` | `PostgresChangeFeed` | `StreamSource<Data>` | a row change | log sequence number |

A Kafka topic of several partitions has no total order across partitions. Its
contents MUST be in each partition's order and interleave partitions by
message time, and its configuration MAY name a single partition when a total
order is wanted.

**Live delivery and work queues.**

| Kind | HQL type | Contract | Element | Why live |
| --- | --- | --- | --- | --- |
| `nats` | `NatsSubject` | `LiveSource<Data>` | a message on a subject or wildcard | core NATS keeps nothing |
| `mqtt` | `MqttTopic` | `LiveSource<Data>` | a publication | only the retained message survives |
| `amqp` | `AmqpQueue` | `LiveSource<Data>` | a delivery | receiving takes it from other consumers |
| `sqs` | `SqsQueue` | `LiveSource<Data>` | a message | receiving hides it until it times out |
| `pubsub` | `PubSubSubscription` | `LiveSource<Data>` | a message | delivery is per subscription |
| `webhook` | `Webhook` | `LiveSource<Data>` | a request body | the host receives it once |

The same server can back two kinds. A NATS subject read live is `nats`; the
JetStream stream that captures that subject is `nats_stream`, and is
replayable.

**HTTP APIs.**

| Kind | HQL type | Contract | Element | Order |
| --- | --- | --- | --- | --- |
| `rest` | `RestResource` | `KeyedSource<String, Data>` | an item of a paginated collection | configured |
| `graphql` | `GraphqlQuery` | `FiniteSource<Data>` | a node of a configured query | configured |
| `grpc` | `GrpcStream` | `StreamSource<Data>` or `LiveSource<Data>` | a response message | by method |
| `feed` | `Feed` | `FiniteSource<Data>` | an RSS or Atom entry | entry time, then id |

A `rest` source reads with `GET` only. Its configuration names the collection
path, the pagination scheme and the key of an item, and `get` reads the item
path. Any other method is an effect.

**Graph stores.**

| Kind | HQL type | Contract | Element | Order |
| --- | --- | --- | --- | --- |
| `neo4j` | `GraphStore` | `Catalog<GraphLabel>` | — | — |
| — | `GraphLabel` | `KeyedSource<Data, Data>` | a node with that label | node id |

A graph store's nodes are read as a source; turning them and their edges into a
`Graph<N, E>` is the `graph` extension's step, as for a vault.

### 9. Run-context bounds

How a host fetches — rows per round trip, pages per request, connection
timeouts, retries — is configuration of the run context and is invisible to a
program, with one rule: a fetch that fails or exceeds a bound is an error, and
MUST NOT appear to the program as fewer elements.

- The run context sets `max_elements` and `max_bytes`, the most of one source
  an evaluation may hold at once. Exceeding either is an evaluation error naming
  the source and the bound; truncating instead would change observable values.
  A program that needs more takes it in portions with `collect`, which holds one
  batch at a time.
- An unbounded stream is bounded by its consumer: a `take` stops it, and a step
  that needs the whole input is a type error by
  [HQL-0006](../HQL-0006/README.md).

## Backwards Compatibility

- `cards` keeps its name and its contents. It is `vault.cards`, a lazy
  `Seq<Card>` in sorted-walk order ([HQL-0020](../HQL-0020/README.md)); this
  record makes `vault` a source and leaves `cards` as it is.
- `source`, `member` and `vault` are new names, and `get` gains keyed sources. A
  program binding one of them shadows it as any binding does.
- [HQL-0001](../HQL-0001/README.md) excludes extension-defined types and
  document sources. This record requires both and MUST be accepted together
  with an amendment to HQL-0001 that lets an extension provide source kinds and
  their types, and nothing else new.

## Security Considerations

- **Secrets.** Program text never contains a credential: ids are the only way to
  a source, and the section "Configuration" refuses literal secrets in configuration. A query can
  therefore be shared, committed and logged.
- **Reach.** A program can read every source its run context configures and no
  other. Configuration is the access-control boundary, and the run context — not
  the program — decides it.
- **Checking is offline.** `hql check` opens nothing, so checking a hostile
  program reveals nothing about the network it would run on.
- **Read-only.** No operation here writes; native query steps run in read-only
  transactions.
- **Resource bounds.** The section "Run-context bounds" bounds what an evaluation holds of every source, so
  a source cannot make an evaluation unbounded except through a `Stream` a
  consumer chose not to stop.
- **Provenance in output.** A `Message` carries `headers`, which can hold
  tokens set by a producer. A presenter SHOULD NOT print headers unless asked.

## Deployment / Activation

1. Declare `std/sources.hql`, and expose the filesystem vault as
   `vault : VaultSource`, with `cards` bound to `vault.cards`; no
   configuration change.
2. Add `[sources.<id>]` and `source(id)` with a second `vault` kind source, so
   two vaults can be read side by side.
3. Amend HQL-0001 so an extension may provide source kinds.
4. Ship one file kind (`csv`) and one database kind (`postgres`) as extensions,
   the second with HQL-0004 pushdown of `filter` and `take`.
5. Ship log kinds only after `Stream<T>` lands by HQL-0006.

## Reference Implementation

- `std/sources.hql` — the declarations of the section "The abstract types"
- `src/sources.rs` — configuration loading, id validation, the secret-key
  refusal and the bounds of the section "Run-context bounds"
- `src/checker.rs` — typing `source(id)` from configuration
- `src/extensions/mod.rs` — a source-kind registration beside step
  registration
- `src/vault.rs` — the `Vault` struct stays the filesystem vault, and becomes
  one implementation of a host-side interface that every `VaultSource` kind
  implements; that interface, like every Rust trait, stays off HQL's surface
- `doc/models/behavior/queries.md` and issue 0010 — `cards` bound to
  `vault.cards` of the source `vault`

## Test Plan

Unit tests MUST include:

- id validation: `field_notes` accepted; `Field-Notes` and a URL refused, the
  URL with the configuration diagnostic
- a configuration with `password = "…"` refused; with `password_env` accepted
- a finite kind with no stated order refused at load
- a stream kind with no `start` refused at load, and a live kind with one
  refused the same way
- `source(name)` with a non-literal argument refused as a type error

Integration tests MUST include:

- two vault sources in one program, each in name order, `take(2)` over
  each identical across runs
- an unknown id, reported with the nearest configured id
- `hql check` on a program naming a `postgres` source succeeding with no
  network available
- a source exceeding `max_elements` failing with the source and the bound
  named, and the same source processed whole through `collect`
- `filter(m => m.time >= d)` over a fixture log giving the same messages with
  and without a seek

```bash
cargo test
cargo test --test corpus
```

## Open Questions

None.

## Changelog

- 2026-10-18: drafted
- 2026-10-19: `cards` is stated as `vault.cards` in sorted-walk order
  (HQL-0020); no part of this record is implemented yet beyond the filesystem
  vault
- 2026-10-22: accepted, implemented in part. `[sources.<id>]` in `hql.toml`
  is read with the vault and refused at load for an id off `snake_case`, the
  id `vault`, an unknown kind, a missing or unknown key and a literal secret.
  `source("id")` is typed at check time from the configuration alone, needs a
  literal id, refuses a connection string and a format, and names the
  nearest configured id. The kinds `vault` (a `Vault` with its own `cards`,
  loaded once per run), `csv`, `json` (the elements of one array),
  `json_lines`, `yaml` and `sqlite` (one table, ordered by `order`, else by
  `rowid`) are implemented; `element` types a file or table's elements, and
  `[execution] max_elements` and `max_bytes` bound a source. `std/io/mod.hql`
  declares `KeyedSource`, `StreamSource`, `Catalog`, `Message` and
  `Position`, and `VaultSource` as [HQL-0020](../HQL-0020/README.md) states
  it, a `DataSource` and not a `Seq<Card>`. A file or table is typed `Seq<T>` rather than a
  concrete source type, `Vault` does not narrow `VaultSource`, and
  `tap("id")`, `member`, `get` over a keyed source, the stream, live and
  network kinds and native queries are not built
