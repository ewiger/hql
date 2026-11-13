# 16. Tables and topics

The shop's events outgrow a file. This chapter moves them three times: into
a SQLite table on this machine, into a PostgreSQL table on a server, and
onto a Kafka topic that the checkout writes to as orders happen. At each
move the configuration changes and the pipeline does not. The last section
is the move every real pipeline makes sooner or later: a topic carrying
messages that are not what the type says.

Every event in the shop is an `Event`, declared once for the chapter:

<!-- define: Event -->
```hql
type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}
```

## A table on this machine

A `sqlite` endpoint is one table in a database file. The shop's ledger is
the table `orders` in `data/shop.db`:

```toml
[endpoint.ledger]
kind = "sqlite"
path = "data/shop.db"
table = "orders"
order = ["id"]
```

`order` names the columns the rows are read in, so two reads give the rows
in the same order. Filling the table is a `target`: the events are read as
`Event`s, and each field becomes a column. The program is shown checked
rather than run, so that the book's tests do not rewrite the table, and
`Step` is the type of a program whose result is what it wrote:

<!-- run: check with Event --vault examples/pipelines/shop -->
```hql
events : Seq<Event> = source("events")
events | target("ledger")
```

```text
Step
```

A table ends, so `source` reads it whole, as a `Seq`, and every step of
[chapter 6](06-sql-like-queries.md) applies. The revenue per customer is a
`group` and a `sum`, and an amount keeps its digits as a `Decimal` on the
way in and on the way out:

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
orders : Seq<Event> = source("ledger")
orders
| filter(o => o.kind == "order_placed")
| group(by = o => o.customer)
| map(g => {customer: g.key, total: g.items | map(o => o.amount) | sum})
```

```text
{{customer: "ada", total: 135.75}, {customer: "grace", total: 42.00}, {customer: "linus", total: 7.99}}
```

`tap("ledger")` gives the same rows one at a time, a `Stream` that ends
after the last row, and holds one row in memory:

<!-- run: --vault examples/pipelines/shop -->
```hql
tap("ledger") | take(2) | map(o => o.id)
```

```text
[1, 2]
```

## The same table on a server

The shop hires a second clerk, and the ledger moves to PostgreSQL, where
both can reach it. The configuration names the database by the variable
holding its connection string; the table and its order are the same:

```toml
[endpoint.orders]
kind = "postgres"
dsn_env = "SHOP_DSN"
table = "orders"
order = ["id"]
```

The revenue report is the program above with `"ledger"` replaced by
`"orders"`. It checks without a database:

<!-- run: check with Event --vault examples/pipelines/shop -->
```hql
orders : Seq<Event> = source("orders")
orders
| filter(o => o.kind == "order_placed")
| group(by = o => o.customer)
| map(g => {customer: g.key, total: g.items | map(o => o.amount) | sum})
```

```text
Set<Data>
```

and run against one, it prints what the SQLite table printed. What changes
is what happens at the edges. The table's columns are read before the first
row and compared with `Event` through the
[schema boundary](09-io/configured.md#the-schema-boundary), so a column
`Event` does not declare, or one of the wrong type, fails before any row is
decoded. Only the columns `Event` reads are selected. A `numeric` column is
a `Decimal` with its digits, `jsonb` the tree it holds, and `NULL` an
absent field.

## A topic

The shop opens a web store. The checkout does not write to the ledger: it
publishes each event to a Kafka topic, `order_events`, and whatever needs
to know about orders reads the topic.

```toml
[endpoint.order_events]
kind = "kafka"
brokers_env = "KAFKA_BROKERS"
topic = "order_events"
idle_ms = 2000
```

A topic is a log that does not end. `tap("order_events")` is a `Stream`,
one JSON document per message, decoded as the element type; the stream
starts at the earliest message the topic keeps, or at the next one
published under `from = "latest"`:

<!-- run: skip -->
```hql
events : Stream<Event> = tap("order_events")
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

The four lines appear as the messages arrive, and the program ends two
seconds after the last one: `idle_ms` bounds how long the stream waits for
the next message. Without it the stream waits without end, which is what a
service that reads a topic does.

Moving data between the two is a pipeline of two steps each way. A table's
rows onto the topic, one message per row, in the table's `order`:

<!-- run: check with Event --vault examples/pipelines/shop -->
```hql
orders : Seq<Event> = source("orders")
orders | sink("order_events")
```

```text
Step
```

and the topic's messages into the table, one row per message, committed as
each arrives:

<!-- run: check with Event --vault examples/pipelines/shop -->
```hql
events : Stream<Event> = tap("order_events")
events | sink("orders")
```

```text
Step
```

These are the first two programs of the
[`kafka-postgres` case](https://github.com/ewiger/hql-demo/tree/main/kafka-postgres)
of hql-demo, which seeds the table, moves its rows through the topic into a
second table, and compares the second with the first.

## What a write promises

The four steps say what is written. The kind says when it is safe.

| | `target` | `sink` |
| --- | --- | --- |
| a `sqlite` or `postgres` table | every row in one transaction: all of them or none | each row committed as it arrives |
| a `kafka` topic | one message per element of a completed collection | one message per value, as each arrives |

`mode` in the configuration states what a write does with what the table
already holds: `create` refuses an existing table, `append` adds after its
rows, and `replace`, the default, empties it and keeps its definition,
indexes and grants. A table that does not exist is created with a column
per field. A topic that does not exist is an evaluation failure: creating a
topic is the cluster's business.

A write replaces or extends what a table holds. Changing rows it holds, in
place, and several such changes as one transaction, are the statements of
[chapter 19's transfer page](19-oltp/transfers.md).

Every message one program writes goes to the topic's first partition, so a
reader is given them in the order they were written. A read records no
position: each run of a program starts where `from` says, so reading a topic
twice gives its messages twice. A reader that resumes where it stopped is a
JetStream consumer, in [chapter 17](17-subjects-and-the-lakehouse.md).

## A topic that lies

Not every message on a topic is an `Event`. The web store has a bug that
sends `"free"` as an amount, an old client leaves out the customer, a
refund is negative, and a new service publishes `order_cancelled`, a kind
nobody has agreed on. Reading the topic as `Stream<Event>` is strict: the
first such message stops the evaluation, naming the endpoint and the
message.

A pipeline that must keep running reads the messages as `Data`, so none
fails to decode, and states what an order is as a contract
([validation as a value](11-advanced-data-typing/validation-as-a-value.md)):

<!-- define: EventData -->
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
```

`validate(EventData)` gives one `Result` per message: the order, or the
message as it arrived with every rule it broke. `fork` gives every branch the
same input, and `oks()` and `errs()` keep each side. The producer's eight
events are in a file as well, so the policy can be tried on this machine
before it meets the topic:

<!-- run: with EventData --vault examples/pipelines/shop -->
```hql
source("raw_events")
| validate(EventData)
| fork {
    clean    = oks()  | map(e => e.id),
    rejected = errs() | map(e => {id: e.value.id, rules: e.violations | map(v => v.rule)}),
}
```

```text
{clean: [1, 2, 4, 6], rejected: [{id: 3, rules: ["EventData.amount.type"]}, {id: 5, rules: ["EventData.customer.missing"]}, {id: 7, rules: ["EventData.non_negative"]}, {id: 8, rules: ["EventData.known_kind"]}]}
```

The production pipeline is the same shape over the topic, with each branch
ending in a table. The clean side is read into a closed `Event`, since a
table is written from a declared record; the rejected side keeps the
message as the producer sent it, so it can be repaired and sent again:

<!-- define: Rejected -->
```hql
type Rejected {
    id      : Int
    message : Data
    rules   : List<String>
}
```

<!-- run: check with Event,EventData,Rejected --vault examples/pipelines/shop -->
```hql
tap("order_events_raw")
| validate(EventData)
| fork {
    clean =
        oks()
        | map(e => Event(id = e.id, kind = e.kind, customer = e.customer, amount = e.amount, at = e.at))
        | sink("order_clean"),
    quarantine =
        errs()
        | map(e => Rejected(
            id = match e.value.id { some(Int n) => n, _ => 0 },
            message = e.value,
            rules = e.violations | map(v => v.rule)))
        | sink("order_quarantine"),
}
```

```text
{clean: Step, quarantine: Step}
```

Each message goes to exactly one branch, in the order the topic holds them,
and neither branch ends the other. After a run, the quarantine table holds:

<!-- run: skip -->
```hql
rejected : Seq<Rejected> = source("order_quarantine")
rejected | map(r => {id: r.id, rules: r.rules})
```

```text
[{id: 3, rules: ["EventData.amount.type"]}, {id: 5, rules: ["EventData.customer.missing"]}, {id: 7, rules: ["EventData.non_negative"]}, {id: 8, rules: ["EventData.known_kind"]}]
```

Each row's `message` is the tree the producer sent, `{amount: "free", …}`
for the first, so a repaired message is the same tree with one field
changed.

The contract is the policy: what an order is. The shape of the pipeline is
what happens to a message that is not one. Changing either is a change to
one declaration, and the checker checks both before a message is read.

## What is next

[Chapter 17](17-subjects-and-the-lakehouse.md) gives the shop a message bus
that keeps its place, and a lakehouse whose tables are files in object
storage: the same pipeline, with a subject in place of the topic and Delta
tables in place of PostgreSQL.
