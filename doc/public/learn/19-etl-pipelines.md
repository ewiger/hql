# 19. ETL pipelines

Every data platform has a language for the wiring. Data Factory wires
activities in JSON, PySpark in a DataFrame API, ksqlDB in SQL over topics,
Kafka Streams in Java, dbt in SQL models with a dependency graph around them.
Each describes the same four things: where data comes from, what is done to
it, where it goes, and when. They differ in where a mistake is found: in a
validator before deployment, in a cluster at run time, or in a dashboard the
next morning.

HQL's claim is that the wiring is a program like any other in this book. A
pipeline is one `.hql` file. What it reads and writes are endpoint ids that
configuration binds to systems. Its types are checked before anything
connects. And the same text runs over a subject that never ends and over the
archive of what the subject once carried.

The shop of chapters [14](14-pipelines.md) to
[18](18-apis-webhooks-and-stores.md) has every piece. This chapter puts them
into one file, then follows the file to where it fails, to where it is
replayed, and to where it may run.

## The shop's day in one file

Orders arrive on the JetStream subject `orders.raw`. Each is appended to an
archive file as it arrived. Each is validated: the valid ones land in the
silver Delta table, and a placed order of 100 or more is also published as
an alert to a core NATS subject; the invalid ones go to the dead-letter
subject with the rules they broke. The configuration is the shop's
`hql.toml`; the pipeline, `day.hql`, is this:

<!-- run: check --vault examples/pipelines/shop -->
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

type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

type Rejected {
    id      : Int
    message : Data
    rules   : List<String>
}

tap("orders_raw")
| tee archived = sink(file("archive/orders.jsonl").end, Json)
| validate(EventData)
| fork {
    silver =
        oks()
        | map(e => Event(id = e.id, kind = e.kind, customer = e.customer, amount = e.amount, at = e.at))
        | tee large =
            filter(e => e.kind == "order_placed" and e.amount >= 100)
            | map(e => {customer: e.customer, amount: e.amount, text: fmt("large order from {}", e.customer)})
            | sink("alerts")
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
{silver: Unit, rejected: Unit}
```

Read top to bottom, it is the dataflow diagram a platform would draw:

```text
orders.raw ──┬──▶ archive/orders.jsonl
             │
             └──▶ validate ──ok──┬──▶ orders_silver
                     │           └──▶ alerts.orders   (≥ 100, placed)
                     └──err─────────▶ orders.rejected
```

One input, three kinds of fan-out, four outputs. `tee` branches a pipeline
without ending it: the archive receives every order before validation, and
the alert branch receives each valid order before the silver table does.
`fork` sends every value to every branch, and `oks()` and `errs()` keep one
side each. A branch of either is typed like the trunk, so a branch that
ended in `target` over this stream would be refused, as any `target` of a
stream is.

There is no orchestration around the file: no activity names, no cluster,
no connection strings. `hql check day.hql` reads `hql.toml`, resolves every
id to its kind and every kind to a type, checks the whole file, and opens
no connection; the result above is what it prints. Every system in the file
is in the current release, and the
[`nats-lakehouse` case](https://github.com/ewiger/hql-demo/tree/main/nats-lakehouse)
of hql-demo runs the same shape against NATS and a lakehouse.

The gold report of [chapter 17](17-subjects-and-the-lakehouse.md) is the
second file: it reads the silver table whole when a report is wanted, and
replaces the gold table in one commit. The two share no state but the silver
table.

## When a mistake is found

A binding can break, and it breaks at the earliest phase that can see it.
Checking sees the configuration; evaluation sees the systems:

| What is wrong | Found | What is said |
| --- | --- | --- |
| `tap("order_event")` | checking | check error[unknown-name]: no endpoint `order_event` is configured; did you mean `order_events`? |
| `source("order_events")` | checking | check error[type-mismatch]: `order_events` is a `kafka` endpoint, read by `tap` as a `Stream`, not by `source` as a `Seq` |
| a stream written by `target("orders")` | checking | check error[type-mismatch]: `target("orders")` writes a Table or a sequence of a declared record, not Stream&lt;Event&gt; |
| `sink("ledger")` | checking | check error[type-mismatch]: endpoint `ledger` is a `sqlite` endpoint, which `sink` does not write; write it whole with `target` |
| a password in `hql.toml` | loading the vault | `url` holds credentials; name the environment variable that holds the address, `url_env = "VARIABLE"` |
| `SHOP_DSN` not set | evaluation, before the first row | execute failure[io-failure]: endpoint `orders`: the environment variable `SHOP_DSN`, which `dsn_env` names, is not set |
| a raw order read as `Event` | evaluation, at that order | execute failure[io-failure]: endpoint `raw_events`, field `amount`: "free" is not a decimal number |
| a write to a wildcard subject | evaluation, at the write | execute failure[io-failure]: subject `orders.>` holds a wildcard, which reads many subjects and writes none; a message is published to one subject |

A secret is named by the variable that holds it and never printed. A
failure at evaluation names the endpoint by its id and the value by its
field; what it never does is continue past a value it could not read and
count it later as missing. A pipeline that must continue says so in its
shape, as `day.hql` does with `validate` and `fork`: the order that would
have stopped it is a row in the dead-letter subject instead.

## A replay is the same file

The pipeline names one input. Bound to a function, it takes any stream of
`Data`, and the input is chosen where it is called:

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

<!-- run: with EventData --vault examples/pipelines/shop -->
```hql
route = (raw: Stream<Data>) => raw
| validate(EventData)
| fork {
    clean    = oks()  | map(e => fmt("ok   {}", e.id)) | sink(stdout, Text),
    rejected = errs() | map(e => fmt("err  {} {}", e.value.id, e.violations | map(v => v.rule))) | sink(stdout, Text),
}

tap("raw_events") | route
```

```text
ok   1
ok   2
ok   4
ok   6
err  3 ["EventData.amount.type"]
err  5 ["EventData.customer.missing"]
err  7 ["EventData.non_negative"]
err  8 ["EventData.known_kind"]
{clean: (), rejected: ()}
```

`tap` over a file is a `Stream` that ends after the file's last order, so
the routing written for a subject runs over a file of recorded messages,
with no broker at all. That is a test. The archive `day.hql` writes is such
a file, so the same call over `tap(file("archive/orders.jsonl"))` is a
**replay**: last week's orders through this week's rules. And the call over
`tap("orders_raw")` is production. The platform words, test, replay,
backfill, name three inputs to one pipeline, not three pipelines.

The other direction is as short. A stream becomes something finite by
bounding it: `take(n)` for a count, `collect(n)` for batches of a size,
each batch a `List` every whole-input step of chapter 6 applies to:

<!-- run: check --vault examples/pipelines/shop -->
```hql
tap("orders_raw") | take(100) | size
```

```text
Int
```

A **snapshot** is the first hundred, or the first day, of a stream written
whole by `target`; a **micro-batch** is `collect(n)` and one write per batch.

## Streams against tables

!!! warning "Design only"

    This section describes functionality that is not in the current release.
    It is written as if it existed: these programs do not run, and names and
    types may change before they ship.

The commonest enrichment is a stream against a lookup: each order joined to
its customer's record in a table. The finite side is read whole; the live
side passes through one value at a time, and no value waits for another, so
`join` takes a `Stream` on the left and a `Seq` on the right:

<!-- run: skip -->
```hql
type Customer { customer : String, region : String, vip : Bool }

customers : Seq<Customer> = source("customers")

tap("orders_raw")
| Event
| join(customers, left = o => o.customer, right = c => c.customer,
       into = (o, c) => {id: o.id, region: c.region, vip: c.vip, amount: o.amount})
| filter(x => x.vip)
| sink("vip_orders")
```

Two streams joined would wait for each other without end, and the checker
says so: `join` over two `Stream`s is refused until one of them is bounded.
A **window** is `collect` by time instead of by count, a
`Stream<List<T>>` whose batches close when the span does:

<!-- run: skip -->
```hql
tap("orders_silver_changes")
| window(1d, by = e => e.at)
| map(day => day
    | group(by = e => e.customer)
    | map(g => Revenue(customer = g.key, orders = g.items | size, total = g.items | map(e => e.amount) | sum)))
| emit
| sink("daily_revenue")
```

Inside a window every whole-input step applies, which is how a daily total
is a `group` and a `sum` over an input that never ends.

## Partitions, schedules and backends

Three things a platform configures stay out of the file.

A **partition** is the logical unit of ownership. `partition(by = e =>
e.customer)` is in the current release: it states that the orders of one
customer belong together, keeps every value and its type, and on
`localhost` changes nothing else. It names no worker and no Kafka partition.
It is what a backend reads to know where a per-customer `group` may run
without the parts having to meet.

!!! warning "Design only"

    The rest of this section describes functionality that is not in the
    current release. It is written as if it existed: names and keys may
    change before it ships.

A **schedule** is configuration beside the endpoints, `[jobs.report] run =
"report.hql"` with an `at`, or a trigger that is itself a source: a subject
whose messages each start a run. The file does not know when it runs.

A **backend** is the execution strategy of [chapter 14](14-pipelines.md).
`localhost` evaluates the checked program step by step in one process, and
is the one backend in the current release. A distributed backend, on Spark,
Flink or a Databricks cluster, runs the pure steps between two effects
where the data is: the `filter` and `map` of a branch on whichever worker
holds the value, a `group` after a `partition` on the worker that owns the
key. It may not move an effect, run one twice, or change an answer: a
backend computes what `localhost` computes, at the same type, with the same
failure, and refuses before running any of a program it cannot run.

```toml
[execution]
backend = "spark"
```

That is the difference between the `spark` kind of chapter 17 and a Spark
backend. The kind is a table the program reads and writes; the backend is a
place the program runs. A program names neither its backend nor its
schedule, so the file a laptop checks is the file the cluster runs.

## What each of them wires

| | Sources and sinks | Transformations | Where a mistake is found |
| --- | --- | --- | --- |
| Data Factory | linked services and datasets, in JSON | activities, each a box | at validation of the JSON, or in the run's monitor |
| PySpark | readers and writers, in code | DataFrame operations | when the job runs, at the operation that fails |
| ksqlDB | topics as streams and tables | SQL over them | when the statement is submitted, or in the records it skips |
| dbt | models over a warehouse | SQL, with tests after | when the model builds, or when its tests run |
| HQL | ids bound in `hql.toml` | the steps of this book | when the file is checked, before anything connects |

The columns are the same four things. What HQL adds is a type for each
arrow: stream against table, finite against live, a declared record against
a configured boundary, a contract against what a producer actually sent,
each checked where the file is written, so that the file that passed is the
file that runs.

## What is next

Part III reads a different kind of data. [Chapter 20](20-a-vault.md) opens a
folder of documents and asks it the questions chapter 6 asked a table.
