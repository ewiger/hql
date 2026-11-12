# A replay is the same file

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
