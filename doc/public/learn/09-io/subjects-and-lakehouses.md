# Subjects and lakehouse tables

A `nats` endpoint is one core NATS subject, a `nats_jetstream` endpoint is
one subject of a NATS JetStream stream, and a `spark` endpoint is one table
behind a Spark SQL endpoint: a Databricks SQL warehouse, or an open-source
Spark Thrift Server over Delta Lake tables.
They are configured as a [topic and a table](configured.md#topics-and-tables)
are, and they sit on the same two sides of the line `source` and `tap` draw:
a subject is a log that does not end, and a table ends.

| Kind | What it is | Read by | Written by |
| --- | --- | --- | --- |
| `kafka` | one topic | `tap`, as a `Stream` | `sink`, `target` |
| `nats` | one core subject, kept by no one | `tap`, as a `Stream` | `sink`, `target` |
| `nats_jetstream` | one subject of a JetStream stream | `tap`, as a `Stream` | `sink`, `target` |
| `postgres` | one table | `source`, as a `Seq` | `target`, `sink` |
| `spark` | one table of a lakehouse | `source`, as a `Seq` | `target`, `sink` |

The programs on this page need a server each, so none of them runs here.
Each runs in the
[hql-demo repository](https://github.com/ewiger/hql-demo), which starts NATS,
MinIO and a Spark Thrift Server with Docker Compose and compares what the
programs print with what this page shows.

## A core subject

NATS delivers a message to whoever is subscribed to its subject when it is
published, and keeps nothing. A `nats` endpoint names the server and a
subject:

```toml
[endpoint.orders]
kind = "nats"
url_env = "NATS_URL"
subject = "orders.>"
idle_ms = 3000
```

The address is written as `url`, or held by the environment variable
`url_env` names. An address may hold a user and a password, and one that does
is refused in `hql.toml`, so it is never written there.

`tap("orders")` subscribes when the stream is first traversed, and is given
every message published from then on, in the order it arrives. A message
published before is not there to be given, so a reader must be subscribed
before the writer publishes; nothing is read from the earliest message, and
the endpoint takes no `from`. A subject may hold the wildcards `*`, one
token, and `>`, every token that follows, so `orders.>` reads every subject
under `orders.`:

<!-- run: skip -->
```hql
events : Stream<Event> = tap("orders")
events
| filter(e => e.kind == "order_placed")
| map(e => {id: e.id, customer: e.customer, amount: e.amount})
```

`sink` publishes each value as one message to one subject, so an endpoint
with a wildcard is read only. Nothing confirms a core message: `sink` ends
once the server has the messages, whether or not anyone was subscribed.

## A subject of a stream

JetStream is the part of the server that keeps messages: a **stream** stores
every message published to the subjects it is configured with, in order. A
`nats_jetstream` endpoint names the server, the stream, and one subject
within it:

```toml
[endpoint.events]
kind = "nats_jetstream"
url_env = "NATS_URL"
stream = "EVENTS"
subject = "events.orders"
idle_ms = 2000
```

`sink("events")` publishes each value as one message, a JSON document. A
message is written once the stream answers that it stored it, so a program
that ends without a failure has lost nothing:

<!-- run: skip -->
```hql
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

events : Seq<Event> = source("mock_events")
events | sink("events")
```

A subject that no stream stores is an evaluation failure, not a message that
disappears. Creating the stream is the server's business, as creating a topic
is a cluster's.

`tap("events")` reads the stream's messages under that subject in the order
the stream holds them, from the earliest it keeps, or from the next one
published under `from = "latest"`. It is a `Stream`, and `source("events")`
is a type error, as it is for a topic:

<!-- run: skip -->
```hql
events : Stream<Event> = tap("events")
events
| filter(e => e.kind == "order_placed")
| map(e => {id: e.id, customer: e.customer, amount: e.amount})
```

```text
{amount: 120.50, customer: "ada", id: 1}
{amount: 42.00, customer: "grace", id: 2}
{amount: 7.99, customer: "linus", id: 4}
{amount: 15.25, customer: "ada", id: 6}
```

The read leaves nothing on the server: no position is recorded, so reading a
subject twice gives the same messages twice. Without `idle_ms` the stream
waits for messages without end; with it, the stream ends once none has
arrived for that many milliseconds.

A subject may hold the wildcards `*`, one token, and `>`, every token that
follows. `subject = "events.>"` reads every subject of the stream under
`events.`. Such an endpoint is read only: a message is published to one
subject, and `sink` to a wildcard is an evaluation failure that says so.

## A position the server keeps

A stream stores messages. A **consumer** is the server's record of how far
one reader has come in a stream, and a durable consumer is one the server
keeps under a name. An endpoint that names one reads from where the read
before it ended:

```toml
[endpoint.billing]
kind = "nats_jetstream"
url_env = "NATS_URL"
stream = "EVENTS"
subject = "events.orders"
consumer = "billing"
idle_ms = 2000
```

<!-- run: skip -->
```hql
events : Stream<Event> = tap("billing")
events | map(e => e.id)
```

The first run reads every event the stream keeps:

```text
1
2
3
4
5
6
```

A second run prints nothing, and a run after two more events are published
prints those two. The first read under a name creates the consumer, starting
where `from` says; every later read finds it, and `from` has no effect then.
Two endpoints with different consumers each keep their own position over the
same messages. A consumer keeps one position over one subject, so naming it
with another subject is an evaluation failure.

A message is acknowledged once the program asks for the one after it, and
the last one when the stream ends. A message is therefore delivered at least
once: a program that fails while it holds a message, or that stops early as
`take(2)` does, is given that message again by the next run.

<!-- run: skip -->
```hql
tap("billing") | take(2) | map(e => e.id)
```

```text
[1, 2]
```

The next run of the program above starts with `2`.

## A subject without configuration

Each kind is also a transport type with a constructor, in a module of its
own: `Nats` and `nats(...)` in `io.msg.nats`, `NatsJetStream` and
`nats_jetstream(...)` in `io.msg.nats_jetstream`. A constructor's parameters
are its kind's keys, so a program reaches a subject with no `hql.toml`:

<!-- run: skip -->
```hql
import io.msg.nats_jetstream

orders = nats_jetstream("events.orders", stream = "EVENTS", url_env = "NATS_URL", idle_ms = 2000)

tap(orders) | filter(e => e.kind == "order_placed") | map(e => e.id)
```

```text
1
2
4
6
```

A message is one document, so the format may be left out and is then `Json`.
Both are transports on the network that carry messages, which is what their
supertypes state:

<!-- run: check -->
```hql
import io.msg.nats

nats("orders.order_placed", url_env = "NATS_URL") | is(Network)
```

```text
Bool
```

A module is imported by its full path, and naming it in the program's first
lines is how a program states that it reaches NATS. Each module declares only
its own transport: `nats(...)` takes no `stream`, and `import io.msg.nats`
does not bring `nats_jetstream` into scope. The address is
still `url` or the variable `url_env` names: a constructor takes no secret,
and a `url` that holds credentials is refused.

A constructed transport has no schema boundary: `fields`, `mode` and
`strictness` are stated only in configuration. A message is read as `Data`,
or as the type an annotation names. The boundary of a configured endpoint
writes a `Decimal` as JSON text so that its digits survive, and reads it
back; to a constructed transport that field is text.

## A lakehouse table

A lakehouse keeps tables as files in object storage, with a transaction log
beside them, and answers SQL over them. Databricks is one; Apache Spark with
Delta Lake over an S3 bucket is another, built from open-source parts. Both
answer the same protocol at an HTTP address, so one kind reaches either:

```toml
[endpoint.events]
kind = "spark"
url_env = "LAKEHOUSE_URL"
table = "default.events"
order = ["id"]
mode = "replace"

[endpoint.events.fields.amount]
type = "DECIMAL(12,2)"
```

| Where | `LAKEHOUSE_URL` | `token_env` |
| --- | --- | --- |
| A Databricks SQL warehouse | `https://<workspace host>/sql/1.0/warehouses/<warehouse id>` | names the variable holding a personal access token |
| A Spark Thrift Server in HTTP mode | `http://<host>:10001/cliservice` | left out |

A table name holds its catalog and schema where it has them,
`main.sales.events`. `token` is a secret key: written in `hql.toml` it
refuses the vault, naming `token_env`.

`target("events")` writes a `Seq` or a `Table` of a declared record. A table
that does not exist is created, a column per field: `BIGINT` for an `Int`,
`DOUBLE` for a `Float`, `DECIMAL(38,18)` for a `Decimal`, `BOOLEAN` for a
`Bool`, and `STRING` otherwise. A `DECIMAL` column states how many digits it
holds, so the boundary above names the type `amount` is written as, and the
amounts read back with two decimal places. A list or a record field has no
column type and is a check error:

<!-- run: skip -->
```hql
events : Seq<Event> = source("mock_events")
events | target("events")
```

Rows are written in statements of up to 500, and each statement is one
commit to the table's log. A `target` of up to 500 rows is therefore atomic,
and under `mode = "replace"` it puts its rows in place of the ones the table
held in that same commit, so a reader never sees the table empty.

`source("events")` reads the table through the
[schema boundary](configured.md#the-schema-boundary), as a `postgres` table
is read: the columns are compared with the record before any row is decoded,
and only the columns the record reads are selected. A `DECIMAL` column reads
as a `Decimal` with its digits and a `NULL` as an absent field:

<!-- run: skip -->
```hql
events : Seq<Event> = source("events")
events
| filter(e => e.kind == "order_placed")
| group(by = e => e.customer)
| map(g => {customer: g.key, orders: g.items | size, total: g.items | map(e => e.amount) | sum})
```

```text
{{customer: "ada", orders: 2, total: 135.75}, {customer: "grace", orders: 1, total: 42.00}, {customer: "linus", orders: 1, total: 7.99}}
```

The endpoint supplies the rows of one table; `hql` evaluates the filter and
grouping locally. A future execution backend could translate supported
pipeline steps to Spark SQL. [Logical steps and physical plans](../14-planning-and-backends.md)
explains that design direction and how engine planning fits into it.

## From a subject to a lakehouse

The two meet in the pipeline most platforms have: events arrive on a bus,
the valid ones land in a table, the invalid ones go somewhere they can be
repaired, and a second table holds what is reported.

```text
producer ──▶ orders.raw ──▶ validate ──ok───▶ orders_silver ──▶ customer_revenue
             (subject)         │              (Delta table)     (Delta table)
                               └──err──▶ orders.rejected
                                         (subject)
```

One stream holds both subjects, and two tables hold the rest:

```toml
[endpoint.orders_raw]
kind = "nats_jetstream"
url_env = "NATS_URL"
stream = "ORDERS"
subject = "orders.raw"
idle_ms = 2000

[endpoint.orders_rejected]
kind = "nats_jetstream"
url_env = "NATS_URL"
stream = "ORDERS"
subject = "orders.rejected"
idle_ms = 2000

[endpoint.orders_silver]
kind = "spark"
url_env = "LAKEHOUSE_URL"
table = "default.orders_silver"
order = ["id"]
mode = "append"

[endpoint.customer_revenue]
kind = "spark"
url_env = "LAKEHOUSE_URL"
table = "default.customer_revenue"
order = ["customer"]
```

**Streaming.** The raw subject is read as `Data`, so no message fails to
decode. [`validate`](../11-advanced-data-typing/validation-as-a-value.md)
gives one `Result` per message, and `fork` routes each to one of two sinks,
in order, ending neither:

<!-- run: skip -->
```hql
type EventData : DataContract = {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String

    require amount >= 0 as non_negative
    require ["order_placed", "order_shipped", "payment_failed"] | contains(kind) as known_kind
}

type Rejected {
    id      : Int
    message : Data
    rules   : List<String>
}

tap("orders_raw")
| validate(EventData)
| fork {
    silver =
        oks()
        | map(e => Event(id = e.id, kind = e.kind, customer = e.customer, amount = e.amount, at = e.at))
        | sink("orders_silver"),
    rejected =
        errs()
        | map(e => Rejected(
            id = match e.value.id { some(Int n) => n, _ => 0 },
            message = e.value,
            rules = e.violations | map(v => v.rule)))
        | sink("orders_rejected"),
}
```

`sink("orders_silver")` appends the rows as they arrive, a commit each 500
rows and one at the end. `sink("orders_rejected")` publishes each rejected
message as the producer sent it, with the rules it broke, so the dead-letter
subject is itself a stream a later program taps:

<!-- run: skip -->
```hql
rejected : Stream<Rejected> = tap("orders_rejected")
rejected | map(r => {id: r.id, rules: r.rules})
```

```text
{id: 3, rules: ["EventData.amount.type"]}
{id: 5, rules: ["EventData.customer.missing"]}
{id: 7, rules: ["EventData.non_negative"]}
{id: 8, rules: ["EventData.known_kind"]}
```

**Transformation.** The silver table ends, so it is read whole, and the
report is written whole over the one before it:

<!-- run: skip -->
```hql
type Revenue {
    customer : String
    orders   : Int
    total    : Decimal
}

events : Seq<Event> = source("orders_silver")
events
| filter(e => e.kind == "order_placed")
| group(by = e => e.customer)
| map(g => Revenue(customer = g.key, orders = g.items | size, total = g.items | map(e => e.amount) | sum))
| target("customer_revenue")
```

The two programs share no state but the table between them. The first runs
for as long as messages arrive; the second runs when a report is wanted, and
reads whatever the first has committed by then.

## What is not here

One reader reads through one consumer: several programs sharing a consumer
to divide its messages is not supported. Credential files for NATS are not
read. A lakehouse table holds no nested column and has no constructor yet,
and a Databricks endpoint is reached with a personal access token only.
