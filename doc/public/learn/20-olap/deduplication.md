# Deduplication

A sum over the replay is wrong, and the amount it is wrong by is the whole
motivation for this page:

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
rows : Seq<Event> = source("silver_replayed")
rows | filter(r => r.kind == "order_placed") | map(r => r.amount) | sum
```

```text
253.73
```

The right answer is 188.49: orders 2 and 4 are counted twice, and order 6
is counted at both its amounts. Three kinds of duplicate are in those nine
rows, and each has its own step.

## Exact duplicates

A row delivered twice, identical in every field, is removed by `distinct`,
which keeps each value once at its first position:

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
rows : Seq<Event> = source("silver_replayed")
rows | distinct | map(r => r.id)
```

```text
[1, 2, 3, 4, 5, 6, 6]
```

That is what at-least-once delivery produces: the subject of
[chapter 17](../17-subjects-and-the-lakehouse.md) delivers a message again
when a reader stopped before acknowledging it, and the message is the same
bytes both times. `distinct` is the whole answer where every copy is exact.
It is not the answer for order 6, whose two copies differ.

## Duplicates by key

Two rows with one key are the same fact, and one of them is the one to
keep. Which one is a rule the pipeline states. For a correction, the latest
by `at`:

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
rows : Seq<Event> = source("silver_replayed")
rows
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| sort(by = r => r.id)
| map(r => {id: r.id, amount: r.amount, at: r.at})
```

```text
[{amount: 120.50, at: "2026-11-01T09:15:00Z", id: 1}, {amount: 42.00, at: "2026-11-01T09:20:00Z", id: 2}, {amount: 42.00, at: "2026-11-01T09:21:00Z", id: 3}, {amount: 7.99, at: "2026-11-01T10:02:00Z", id: 4}, {amount: 120.50, at: "2026-11-02T08:00:00Z", id: 5}, {amount: 18.00, at: "2026-11-02T11:50:00Z", id: 6}]
```

Read step by step: `group(by = r => r.id)` is one group per key, each
holding every row with that key in input order; `max_by(r => r.at)` picks
the row with the latest timestamp out of each group, an `Option` since a
group could be empty; `somes` unwraps; `sort` puts the keys in order, since
a set of groups has none. Six rows, order 6 at its corrected amount.

In SQL this is the pattern every warehouse carries:

```sql
select * from (
  select *, row_number() over (partition by id order by at desc) as n
  from silver_replayed
) where n = 1
```

`partition by id` is `group(by = r => r.id)`, `order by at desc` with
`n = 1` is `max_by(r => r.at)`, and nothing is numbered, because the row
wanted is named by what makes it the one to keep rather than by its position
after a sort. Keeping the first version instead, the fact as first reported,
is `min_by`:

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
rows : Seq<Event> = source("silver_replayed")
rows
| group(by = r => r.id)
| map(g => g.items | min_by(r => r.at))
| somes
| filter(r => r.id == 6)
| map(r => r.amount)
```

```text
{15.25}
```

`max_by` and `min_by` take any `Orderable` key, so the rule may be a
timestamp, a version number, or a comparison of two rows written out. Where
two copies share the key and the timestamp, as the exact duplicates do,
either one is kept, and they are the same row.

The sum, over the deduplicated rows:

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
rows : Seq<Event> = source("silver_replayed")
rows
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| filter(r => r.kind == "order_placed")
| map(r => r.amount)
| sum
```

```text
188.49
```

## In the load

Deduplication belongs in the load that writes silver, once, at the grain of
a fact. A batch load reads the replay whole, deduplicates and writes. It
writes to the lakehouse, so it is shown checked: `Unit` is what `hql check`
prints for a pipeline that ends in `target`, a program whose result is what
it wrote:

<!-- run: check with Event --vault examples/pipelines/shop -->
```hql
rows : Seq<Event> = source("silver_replayed")
rows
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| sort(by = r => r.id)
| target("orders_silver")
```

```text
Unit
```

A streaming load cannot group a stream that never ends, and the checker
refuses it, as [chapter 14](../14-etl-pipelines.md) showed for a sum. It
deduplicates within each micro-batch instead, and `sort` makes each batch a
`List` of the declared record, which is what `sink` writes as rows:

<!-- run: check with Event --vault examples/pipelines/shop -->
```hql
events : Stream<Event> = tap("orders_raw")
events
| collect(500)
| map(batch => batch
    | group(by = r => r.id)
    | map(g => g.items | max_by(r => r.at))
    | somes
    | sort(by = r => r.id))
| emit
| sink("orders_silver")
```

```text
Unit
```

Two copies that land in different batches are not caught by this program,
and that is a limit of what a bounded window can know. The [merge
page](merge.md) is the complement: deduplication against what the table
already holds.
