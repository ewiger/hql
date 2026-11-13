# Bronze, silver and gold

A **lakehouse** keeps tables as files in object storage, with a transaction
log beside them, and answers SQL over them. Databricks is one; Apache Spark
with Delta Lake over an S3 bucket is another, built from open-source parts,
and the `spark` kind of [chapter 17](../17-subjects-and-the-lakehouse.md)
reaches either. What a lakehouse holds is the shop's analytical side, and
data platforms name its layers by their metals.

**Bronze** is what arrived, as it arrived. **Silver** is what passed the
checks, typed, one row per fact. **Gold** is what a report reads, already
aggregated at the grain the report asks. Each layer has a load rule, and the
rule is what the layer promises:

| Layer | Holds | Load | Promise |
| --- | --- | --- | --- |
| bronze | every message, raw | append only, never changed | nothing is lost; a silver load can be run again from here |
| silver | every valid fact, typed, once | append the new, deduplicated by key | one row per fact; a sum over it is right |
| gold | one row per report grain | replaced whole by each run | what a reader sees is one run's answer |

In the shop, bronze is the JetStream subject itself, kept by the stream;
silver and gold are Delta tables:

```text
checkout ──▶ orders.raw ──▶ validate ──ok──▶ orders_silver ──▶ customer_revenue
             (bronze)          │             (silver)          (gold)
                               └──err──▶ orders.rejected
                                         (dead letters)
```

```toml
[endpoint.orders_silver]
kind = "spark"
url_env = "LAKEHOUSE_URL"
table = "default.orders_silver"
order = ["id"]
mode = "append"

[endpoint.orders_silver.fields.amount]
type = "DECIMAL(12,2)"

[endpoint.customer_revenue]
kind = "spark"
url_env = "LAKEHOUSE_URL"
table = "default.customer_revenue"
order = ["customer"]
mode = "replace"

[endpoint.customer_revenue.fields.total]
type = "DECIMAL(12,2)"
```

A `DECIMAL` column states how many digits it holds, so the boundary names
the type each amount is written as, and the amounts read back with two
decimal places.

## Bronze to silver

The load is the quarantine of [chapter 16](../16-tables-and-topics.md)
with four ids changed: the subject in place of the topic, a Delta table in
place of the clean table, and a dead-letter subject in place of the
quarantine table.

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

`sink("orders_silver")` appends rows as they arrive, in statements of up to
500, each one commit to the table's log. The dead letters are a subject a
stream keeps, so they are themselves a source another program reads, to
repair or to count:

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

This load keeps the first half of silver's promise, every valid fact and
only those, and not the second: a message the subject delivers twice is
appended twice. The next three pages are the second half.

## Silver to gold

The report reads a table that ends, so it reads it whole, and writes the
gold table over the one before it:

<!-- define: Revenue -->
```hql
type Revenue {
    customer : String
    orders   : Int
    total    : Decimal
}
```

<!-- run: check with Event,Revenue --vault examples/pipelines/shop -->
```hql
events : Seq<Event> = source("orders_silver")
events
| filter(e => e.kind == "order_placed")
| group(by = e => e.customer)
| map(g => Revenue(customer = g.key, orders = g.items | size, total = g.items | map(e => e.amount) | sum))
| target("customer_revenue")
```

```text
Step
```

Under `mode = "replace"` the new rows take the place of the old ones in one
commit, so a reader of the gold table never sees it empty:

<!-- run: skip -->
```hql
revenue : Seq<Revenue> = source("customer_revenue")
revenue
```

```text
[{customer: "ada", orders: 1, total: 120.50}, {customer: "grace", orders: 1, total: 42.00}]
```

Of the eight raw orders, four passed; of those, ada's first and grace's are
the placed ones, and linus's only placed order was the one sent as
`"free"`.

The two programs share no state but the silver table. The first runs for as
long as messages arrive; the second runs when a report is wanted and reads
whatever the first has committed by then. Gold is cheap to throw away and
rebuild, which is why it is replaced; silver is the one copy of each fact,
which is why it is appended to and never replaced; bronze is the record of
what was received, which is why it is never touched at all.
