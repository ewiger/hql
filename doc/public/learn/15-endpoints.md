# 15. Endpoints

Every system in Part II is reached the same way. A file, a table, a topic, a
subject and a webhook differ in where their data is and whether it ends; to
a program each is an **endpoint**, and four steps read and write all of
them. This chapter is that model, told over the shop of
[chapter 14](14-etl-pipelines.md). The programs run with
`--vault examples/pipelines/shop`.

## One file, two spellings

An endpoint is a value that states where documents are. Constructing one
performs no I/O, and a program can reach the shop's events by writing the
endpoint itself, `file(path)`:

<!-- run: --vault examples/pipelines/shop -->
```hql
source(file("examples/pipelines/shop/data/events.json")) | size
```

```text
1
```

or by naming the id the shop's `hql.toml` declares it under:

```toml
[endpoint.events]
kind = "json"
path = "data/events.json"
```

<!-- run: --vault examples/pipelines/shop -->
```hql
source("events") | size
```

```text
6
```

The two answers differ because the two spellings say different things. A
`.json` file read by `file` is one JSON document, the whole array, so the
`Seq` holds one value. The `json` kind is a file whose one document is an
array of records, and it gives the array's elements. A constructed endpoint
states where the bytes are and leaves the rest to the program; a configured
endpoint states the system, its documents, and, on the
[schema boundary](09-io/configured.md#the-schema-boundary), the names and
types that system uses.

The rest of Part II uses ids, for the reason the second spelling exists: a
program that names an id is not tied to one machine's paths, one server's
address, or one system. Moving the events from a file into a table is a
change to `hql.toml`, not to the pipeline.

## Four steps

Two questions sort every read and write. Does the endpoint end, and does the
step take the data whole or one value at a time?

| | Whole | One at a time |
| --- | --- | --- |
| **Read** | `source(e)`: a `Seq`, every document, from an endpoint that ends | `tap(e)`: a `Stream`, one document held at a time |
| **Write** | `target(e)`: one completed value, written once | `sink(e)`: one document per value, written as each arrives |

A table ends, so `source` reads it. A topic does not, so it is read by
`tap`; `source` of one is a check error, because a `Seq` is a value that
ends and a topic has no last message:

<!-- run: --vault examples/pipelines/shop -->
```hql
source("order_events")
```

```text
check error[type-mismatch]: `order_events` is a `kafka` endpoint, read by `tap` as a `Stream`, not by `source` as a `Seq`
 --> <expression>:1:8
  |
1 | source("order_events")
  |        ^^^^^^^^^^^^^^
```

The writing side is the mirror. A stream has no last value for `target` to
wait for, so a stream is written with `sink`:

<!-- run: --vault examples/pipelines/shop -->
```hql
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

events : Stream<Event> = tap("order_events")
events | target("orders")
```

```text
check error[type-mismatch]: `target("orders")` writes a Table or a sequence of a declared record, not Stream<Event>
  --> <expression>:10:10
   |
10 | events | target("orders")
   |          ^^^^^^^^^^^^^^^^
```

<!-- run: check --vault examples/pipelines/shop -->
```hql
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

events : Stream<Event> = tap("order_events")
events | sink("orders")
```

```text
Unit
```

`tap` reads an endpoint that ends as well: `tap("ledger")` gives the table's
rows one at a time, which is how a table larger than memory is read. The
type of each step is fixed before the program runs, so a pipeline that would
wait forever, `tap("order_events") | size`, is a check error and not a
process that hangs ([finite or live](09-io/finite-or-live.md)).

## The kind states the type

`source("orders")` is a PostgreSQL table, and the program above was checked
without a database. Checking reads `hql.toml`, finds `kind = "postgres"`,
and knows from the kind alone that `orders` ends, is read by `source` as a
`Seq` of rows, and is written by `target` in one transaction or by `sink`
row by row. No connection is made until the program runs, and `hql check`
never runs it:

<!-- run: check --vault examples/pipelines/shop -->
```hql
source("customer_revenue")
```

```text
Seq<Data>
```

`customer_revenue` is a Delta table in a lakehouse; the shop's configuration
names it, and that is all the checker needs. The kinds in the current
release, and the chapter each is told in:

| Kind | What one endpoint is | Ends | Chapter |
| --- | --- | --- | --- |
| `csv`, `json`, `json_lines`, `yaml` | a file in that format | yes | [9](09-io/configured.md) |
| `vault` | a folder of documents | yes | [20](21-a-vault.md) |
| `sqlite` | one table of a database file | yes | [16](16-tables-and-topics.md) |
| `postgres` | one table of a PostgreSQL database | yes | [16](16-tables-and-topics.md) |
| `kafka` | one Kafka topic | no | [16](16-tables-and-topics.md) |
| `nats` | one core NATS subject | no | [17](17-subjects-and-the-lakehouse.md) |
| `nats_jetstream` | one subject of a JetStream stream | no | [17](17-subjects-and-the-lakehouse.md) |
| `spark` | one table of a lakehouse, over Spark SQL | yes | [17](17-subjects-and-the-lakehouse.md) |

The id is a literal. A computed id, a connection string in place of an id,
and an id the configuration does not declare are each refused at checking,
on the [endpoints-by-id page](09-io/configured.md#the-id-is-a-literal) of
chapter 9.

## Secrets live in the environment

The shop's `hql.toml` holds no password. An endpoint whose address may carry
a secret names the environment variable that holds it, `dsn_env` for a
database, `brokers_env` for Kafka, `url_env` for NATS and the lakehouse, and
`token_env` for a token:

```toml
[endpoint.orders]
kind = "postgres"
dsn_env = "SHOP_DSN"
table = "orders"
order = ["id"]
```

An address with credentials written into the configuration refuses the whole
vault, before any program is read:

```text
$ hql --vault shop eval '1'
hql: shop: `[endpoint.x]`: `url` holds credentials; name the environment variable that holds the address, `url_env = "VARIABLE"`
```

A variable that is not set is found when the endpoint is first read, and the
failure names the endpoint and the variable, never its value:

```text
$ hql --vault examples/pipelines/shop eval 'source("orders")'
execute failure[io-failure]: endpoint `orders`: the environment variable `SHOP_DSN`, which `dsn_env` names, is not set
 --> <expression>:1:1
  |
1 | source("orders")
```

Every failure past checking has that form: the endpoint by its id, then the
system's own words, then the row or message by its position. A row that does
not decode stops the evaluation there; [chapter 16](16-tables-and-topics.md)
routes such rows instead of stopping.

## Transports

An endpoint that reaches outside the evaluation is a **transport**. Two
properties of a transport matter to a program and to whoever runs it:

- its **locality**: `Direct`, on this machine, such as a file or standard
  input, or `Network`, another process over a connection;
- its **carriage**: what one read moves, bytes that a format splits into
  documents, a framed **message** that is one document, or a **row** of
  named columns that needs no format at all.

`Sqlite` and the two NATS transports are declared with both today. Each is a
module of its own, imported by its full path, and its constructor takes the
keys its kind takes, so a program reaches a subject without any
configuration:

<!-- run: check --vault examples/pipelines/shop -->
```hql
import io.msg.nats_jetstream

tap(nats_jetstream("orders.raw", stream = "ORDERS", url_env = "NATS_URL", idle_ms = 2000))
```

```text
Stream<Data>
```

A message is one document, so no format is named, and it is `Json` unless
one is. A table is rows of named columns, so it takes no format at all, and
naming one is a check error:

<!-- run: check --vault examples/pipelines/shop -->
```hql
import io.db.sqlite

source(sqlite("data/shop.db", table = "orders", order = ["id"]))
```

```text
Seq<Data>
```

A constructor takes no secret, only the name of the variable that holds
one; and a constructed transport has no schema boundary, which only
configuration states. A row is read as `Data`, or as the record an
annotation names, each column decoded as its type holds it, and a record
is written to a constructed table a column per field, replacing the rows
it held. A database file is written whole, by `target`; `sink` writes
rows to a server's table, not to a file's.

!!! warning "Design only"

    The rest of this section describes functionality that is not in the
    current release. It is written as if it existed: names and types may
    change before it ships.

Every kind is a transport with a constructor of its name, and every
transport states its locality and carriage:

| Transport | Locality | Carriage |
| --- | --- | --- |
| `File`, `Stdin`, `Stdout` | `Direct` | bytes |
| `Sqlite` | `Direct` | rows |
| `Http`, `Tcp` | `Network` | bytes |
| `WebSocket`, `Kafka`, `Nats`, `NatsJetStream`, `Rest`, `Graphql`, `Webhook` | `Network` | messages |
| `Postgres`, `Spark` | `Network` | rows |

The carriage settles whether a step takes a format: a byte transport needs
one, a message transport may name one and is `Json` without, and a row
transport takes none, so `source(postgres(dsn_env = "SHOP_DSN", table =
"orders"), Json)` is a check error. The locality is what a host grants: a
document being rendered reaches no `Network` transport, constructed or
configured, and the check reads that from the type, not from a list of
names ([effects and what a host allows](09-io/hosts.md)).

The transports are grouped into modules by the system they reach:

```text
io                     file, stdin, stdout, stderr, source, tap, target, sink
├── net                http, websocket, tcp, rest, graphql, webhook
├── db                 sqlite, postgres, spark, and further databases
├── msg                kafka, nats, nats_jetstream, and further brokers
└── store              s3, azure_blob, gcs, sftp, hdfs
```

`import io.db.postgres` brings `postgres(...)` into scope, as
`import io.db.sqlite` brings `sqlite(...)` and `import io.msg.nats` brings
`nats(...)` today. A configured id needs no
import: the configuration names the kind, and the kind names its module.
A program that constructs a network transport names its module in its
first lines, so what a program may reach is visible before reading it.
[Chapter 18](18-apis-webhooks-and-stores.md) is the `net` and `store`
transports the shop reaches next.

## What is next

[Chapter 16](16-tables-and-topics.md) moves the shop's events out of a file:
into a SQLite table, then a PostgreSQL table, then onto a Kafka topic and
back.
