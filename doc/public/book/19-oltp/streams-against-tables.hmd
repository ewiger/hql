# Streams against tables

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
