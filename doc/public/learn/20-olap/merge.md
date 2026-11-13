# Merge

The write modes of [chapter 16](../16-tables-and-topics.md) are `create`,
`append` and `replace`, and each is about the table as a whole. An
analytical table that is kept up to date needs a fourth, about rows: a row
whose key the table does not hold is inserted, and a row whose key it does
hold is updated. Warehouses call it **upsert** or **merge**, and Delta Lake
calls the statement `MERGE INTO`.

## Merge as a pipeline

Merge is deduplication by key over the union of what the table holds and
what arrives. The current rows and the incoming rows are read, concatenated,
and deduplicated with the same rule as the [previous page](deduplication.md),
and the result replaces the table:

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

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
current  : Seq<Event> = source("ledger")
incoming : Seq<Event> = source("silver_replayed")

current
| concat(incoming)
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| sort(by = r => r.id)
| map(r => {id: r.id, amount: r.amount})
```

```text
[{amount: 120.50, id: 1}, {amount: 42.00, id: 2}, {amount: 42.00, id: 3}, {amount: 7.99, id: 4}, {amount: 120.50, id: 5}, {amount: 18.00, id: 6}]
```

The ledger held six orders; the replay brought nine rows; the result is six
rows, with order 6 updated to its corrected amount and the other five
unchanged. An incoming row with a new id would be inserted by the same
steps, since a group of one keeps its one row. Written back under
`mode = "replace"`, the table is the merge, in one transaction:

<!-- run: check with Event --vault examples/pipelines/shop -->
```hql
current  : Seq<Event> = source("orders_silver")
incoming : Seq<Event> = source("silver_replayed")

current
| concat(incoming)
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| sort(by = r => r.id)
| target("orders_silver")
```

```text
Unit
```

The merge rule, which copy wins, is the `max_by` and nothing else. A rule
that prefers the row already held is `min_by` over the same key; a rule
that prefers the incoming row whatever its timestamp is `last` over each
group, since `concat` put the current rows first.

This reads and rewrites the whole table, which is right for a table of
thousands of rows and wrong for one of billions. The cost is the point of
having a write mode for it. A row changed in place, one at a time as a
transaction does it, is `update` of the
[transfer page](../19-oltp/transfers.md); a merge is the whole table kept
up to date by key.

## Merge as a write mode

!!! warning "Design only"

    This section describes functionality that is not in the current release.
    It is written as if it existed: names and keys may change before it
    ships.

A table's key is a fact about the table, so it is configured beside it, and
`mode = "merge"` makes the key the rule of every write:

```toml
[endpoint.orders_silver]
kind = "spark"
url_env = "LAKEHOUSE_URL"
table = "default.orders_silver"
order = ["id"]
mode = "merge"
key = ["id"]
```

<!-- run: skip -->
```hql
rows : Seq<Event> = source("silver_replayed")
rows
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| sort(by = r => r.id)
| target("orders_silver")
```

`target` sends only the deduplicated rows, and the system does the rest: a
row whose key the table holds replaces that row, and a row whose key it does
not hold is inserted, in one commit. A lakehouse runs it as `MERGE INTO`, a
PostgreSQL table as `INSERT … ON CONFLICT DO UPDATE`, a SQLite table as
`INSERT OR REPLACE`. The program is the one above, and which statement the
kind runs is the kind's business, as a transaction is today.

Two rules follow. A write under `mode = "merge"` to an endpoint with no
`key` is a check error naming the key. And the rows a `target` sends must
themselves be unique by the key, so the deduplication stays in the program:
a merge is told which row to keep among the incoming ones by the pipeline,
and which to keep between an incoming row and a held one by the mode.

`sink` under `mode = "merge"` writes each row as it arrives, inserting or
updating one at a time, which is the streaming load of the previous page
without its blind spot: a copy that lands in a later batch updates the row
the earlier one inserted.
