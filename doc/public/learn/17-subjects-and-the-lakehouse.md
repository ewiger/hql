# 17. Subjects and the lakehouse

The shop's Kafka topic of [chapter 16](16-tables-and-topics.md) carries
every order, and three needs have grown around it. The warehouse wants to
hear about each shipment as it happens and does not care about the ones it
missed. Billing wants every order exactly where it left off yesterday. And
the owner wants reports over all of history without anyone's pipeline
slowing down to produce them.

This chapter meets the three with NATS and a lakehouse. NATS is two kinds:
`nats`, a subject whose messages nobody keeps, and `nats_jetstream`, a
subject of a stream the server keeps. A lakehouse is tables kept as files
in object storage, reached by `kind = "spark"`. Each key of each kind is on
the [subjects and lakehouse tables page](09-io/subjects-and-lakehouses.md)
of chapter 9; this chapter is what the shop does with them.

The programs need servers. They are checked here against the shop's
configuration, and run in the
[hql-demo repository](https://github.com/ewiger/hql-demo), whose
`nats-core`, `nats` and `nats-lakehouse` cases start NATS, MinIO and a Spark
Thrift Server with Docker Compose. The outputs shown are theirs.

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

## Notifications nobody keeps

Core NATS delivers a message to whoever is subscribed to its subject when it
is published, and keeps nothing. That is what a notification is: the
warehouse screen shows what is happening now, and an event from before the
screen was switched on is not news.

The shop publishes each event to a subject named for its kind,
`orders.order_placed`, `orders.payment_failed`, `orders.order_shipped`. The
subject is computed, so the program writes the transport itself, with the
constructor of `io.msg.nats`:

<!-- run: check --vault examples/pipelines/shop -->
```hql
import io.msg.nats

type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

events : Seq<Event> = source("events")

for_kind = (kind: String) => events | filter(e => e.kind == kind)
subject = (kind: String) => nats("orders." + kind, url_env = "NATS_URL")

for_kind("order_placed") | sink(subject("order_placed"))
for_kind("payment_failed") | sink(subject("payment_failed"))
for_kind("order_shipped") | sink(subject("order_shipped"))
```

```text
Step
```

A reader names one subject or a family of them. A subject may hold the
wildcards `*`, one token, and `>`, every token that follows, so the shop's
`notifications` endpoint reads all three:

```toml
[endpoint.notifications]
kind = "nats"
url_env = "NATS_URL"
subject = "orders.>"
idle_ms = 3000
```

<!-- run: skip -->
```hql
events : Stream<Event> = tap("notifications")
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

The subscription starts when the stream is first traversed, so the
`nats-core` case starts this program before the publisher, and a message
published before it would not be there to be given. Nothing confirms a core
message: `sink` ends once the server has the messages, whether or not
anyone was listening. A wildcard endpoint is read only, since a message is
published to one subject.

## A log the server keeps

Billing cannot miss an order. JetStream is the part of the NATS server that
keeps messages: a **stream** stores every message published to its
subjects, in order, and a `nats_jetstream` endpoint names the stream and one
subject in it. The shop's checkout publishes raw orders to `orders.raw`, in
the stream `ORDERS`:

```toml
[endpoint.orders_raw]
kind = "nats_jetstream"
url_env = "NATS_URL"
stream = "ORDERS"
subject = "orders.raw"
idle_ms = 2000
```

<!-- run: check --vault examples/pipelines/shop -->
```hql
source("raw_events") | sink("orders_raw")
```

```text
Step
```

A message is written once the stream answers that it stored it, so a
program that ends without a failure has lost nothing. A subject no stream
stores is an evaluation failure rather than a message that disappears.

`tap("orders_raw")` reads the stream's messages under that subject in the
order the stream holds them, from the earliest it keeps, or from the next
one published under `from = "latest"`. Like a Kafka read, it records no
position: two runs give the same messages twice.

## Billing's place

A **consumer** is the server's record of how far one reader has come in a
stream, and a durable consumer is one the server keeps under a name.
Billing's endpoint is the same subject with a consumer named:

```toml
[endpoint.billing]
kind = "nats_jetstream"
url_env = "NATS_URL"
stream = "ORDERS"
subject = "orders.raw"
consumer = "billing"
idle_ms = 2000
```

The first run through `billing` reads every message the stream keeps. The
next run reads only what was published since; a run with nothing new
prints nothing. Another endpoint with another consumer keeps its own place
over the same messages, so the warehouse's reports and billing's invoices
never move each other's position.

A message is acknowledged once the program asks for the one after it, and
the last one when the stream ends. A message is therefore delivered at
least once: a program that fails while it holds a message, or stops early,
is given that message again by its next run.

<!-- run: skip -->
```hql
tap("billing") | take(2) | map(e => e.id)
```

```text
[1, 2]
```

`take(2)` asked for no third message, so `2` was never acknowledged, and the
next run of this program starts with `2`. A program that writes what it
reads, an invoice per order, writes it so that writing it twice does no
harm.

## The lakehouse

The owner's reports need history, and a broker is not where history is
queried. A **lakehouse** keeps tables as files in object storage, with a
transaction log beside them, and answers SQL over them. Databricks is one;
Apache Spark with Delta Lake over an S3 bucket is another, built from
open-source parts, and the `spark` kind reaches either, by the URL of a
Databricks SQL warehouse or of a Spark Thrift Server:

```toml
[endpoint.orders_silver]
kind = "spark"
url_env = "LAKEHOUSE_URL"
table = "default.orders_silver"
order = ["id"]
mode = "append"

[endpoint.orders_silver.fields.amount]
type = "DECIMAL(12,2)"
```

A lakehouse table ends, so it sits on the table side of the line: read by
`source` through the schema boundary, written by `target` in commits of up
to 500 rows, or by `sink` as rows arrive. A `DECIMAL` column states how
many digits it holds, so the boundary names the type each amount is written
as. The valid orders of the subject above land in this table by the
quarantine of chapter 16 with the ids changed, a subject in place of the
topic and this table in place of the clean one:

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

type Rejected {
    id      : Int
    message : Data
    rules   : List<String>
}
```

<!-- run: check with Event,EventData --vault examples/pipelines/shop -->
```hql
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

```text
{silver: Step, rejected: Step}
```

That table is the shop's first analytical one, and what it is for, the
layers a lakehouse is organized in, and what a report built over it needs
from it are [chapter 20](20-olap/index.md). This chapter's business with
it ends at the transport: a subject in, a table out, and what each
promises.

## Where the work runs

The filter and the grouping above run where `hql` runs. The `spark` endpoint
reaches a table over Spark SQL and sends the statements needed to read or
write it. The surrounding HQL pipeline is evaluated locally.

A future Spark execution backend could translate supported pipeline steps
into SQL and let the engine plan their execution on the cluster. The
companion [Logical steps and physical plans](14-planning-and-backends.md)
explains that handoff and the requirement to preserve the meaning of `|`.

What the current release does not do: several programs sharing one consumer
to divide its messages, NATS credential files, a lakehouse column holding a
list or a record, and a Databricks token other than a personal access
token.

## What is next

[Chapter 18](18-apis-webhooks-and-stores.md) is where the shop's data comes
from outside: a payment provider calling a webhook, a carrier's REST API,
and an archive in object storage. It is design, not yet in the release.
