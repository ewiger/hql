# Windows

`group` makes one group of the rows with one key, and a step after it
reduces each group to one value. Some questions keep every row and add a
value that depends on the row's neighbours: the position of an order among
its customer's orders, the balance after each ledger entry, the change from
the previous revision. In SQL those are window functions, written
`OVER (PARTITION BY ... ORDER BY ...)`. In HQL they are the step `over`.

## One value per row

`over(by, order, into)` partitions a sequence by the key `by` gives each
row, orders each partition by `order`, and calls `into` once per row, with
the row and its `Window`: the row's place in its ordered partition. The
result is a `List` holding what `into` gave for each row, at the row's
input position. No row is merged or dropped.

`row_number` is the row's position in its partition, from 1. Ranking the
replay's rows within each `id`, latest first, and keeping the first of each
is the deduplication of [the previous page](deduplication.md) in its SQL
form, `row_number() over (partition by id order by at desc) = 1`:

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
rows
| over(by = r => r.id, order = (a, b) => compare(b.at, a.at),
       into = (r, w) => {row: r, n: w | row_number})
| filter(x => x.n == 1)
| map(x => {id: x.row.id, amount: x.row.amount})
```

```text
[{amount: 120.50, id: 1}, {amount: 42.00, id: 2}, {amount: 42.00, id: 3}, {amount: 7.99, id: 4}, {amount: 120.50, id: 5}, {amount: 18.00, id: 6}]
```

`order = (a, b) => compare(b.at, a.at)` is a comparison of two rows, here
reversed for latest first; `order` may also be a key, `r => r.at`, for
ascending order. The kept rows are in input order, each at its own row's
position. Without `by`, every row is in one partition.

`group` with `max_by` gives the same six rows, and is the shorter program
when one row per key is all a step wants. `over` is the step when a row's
position is itself the value, or when every row stays.

## Ranks

Rows equal under `order` keep their input order, so `row_number` never
gives two rows one position. `rank` gives rows equal under the order one
rank and skips the positions they share; `dense_rank` does not skip:

```hql
type Score {
    name  : String
    score : Int
}

scores = [
    Score(name = "a", score = 9),
    Score(name = "b", score = 7),
    Score(name = "c", score = 9),
    Score(name = "d", score = 5),
]
scores
| over(order = (x, y) => compare(y.score, x.score),
       into = (s, w) => [w | row_number, w | rank, w | dense_rank])
```

```text
[[1, 1, 1], [3, 3, 2], [2, 1, 1], [4, 4, 3]]
```

`a` and `c` tie at 9. `a` is first in the input, so its `row_number` is 1
and `c`'s is 2; both have rank 1. `b` has rank 3 and dense rank 2.

## A total order

A row kept by its position among equals is kept by where it happened to
arrive. Two revisions of one order written in the same second are equal by
`at`, and `max_by`, like `row_number`, keeps the earlier in the input:

<!-- define: Revision -->
```hql
type Revision {
    id     : Int
    at     : String
    seq    : Int
    amount : Int
}
```

<!-- run: with Revision novault -->
```hql
rows = [
    Revision(id = 6, at = "2026-11-02T11:50:00Z", seq = 1, amount = 15),
    Revision(id = 6, at = "2026-11-02T11:50:00Z", seq = 2, amount = 18),
]
rows
| group(by = r => r.id)
| map(g => g.items | max_by(r => r.at))
| somes
| map(r => r.amount)
```

```text
{15}
```

With the rows in the other order the result is 18. `then` composes two
comparisons, the second deciding where the first is `Equal`; a comparison
that ends in a key no two rows share, here the change sequence `seq`, is a
total order, and the row kept no longer depends on input order:

<!-- run: with Revision novault -->
```hql
rows = [
    Revision(id = 6, at = "2026-11-02T11:50:00Z", seq = 1, amount = 15),
    Revision(id = 6, at = "2026-11-02T11:50:00Z", seq = 2, amount = 18),
]
rows
| group(by = r => r.id)
| map(g => g.items | max_by((a, b) => compare(a.at, b.at) | then(compare(a.seq, b.seq))))
| somes
| map(r => r.amount)
```

```text
{18}
```

## Neighbours

`lag` is the row before in the partition and `lead` the row after, each an
`Option`, `none` past either end. `lag(n)` and `lead(n)` reach `n` rows:

```hql
[1, 4, 9]
| over(order = x => x,
       into = (x, w) => [w | lag | unwrap_or(0), w | lead | unwrap_or(0)])
```

```text
[[0, 4], [1, 9], [4, 0]]
```

## Frames

A frame is a list of rows of the partition, in order. `running` is the rows
from the first up to and including the row, `rows(before, after)` the rows
from `before` positions before to `after` after, clipped at the ends, and
`whole` the partition. An aggregate over a frame is an ordinary step over a
list. The balance after each ledger entry, per account:

```hql
type Posting {
    account : String
    seq     : Int
    amount  : Int
}

ledger = [
    Posting(account = "a", seq = 1, amount = 100),
    Posting(account = "b", seq = 1, amount = 50),
    Posting(account = "a", seq = 2, amount = -30),
    Posting(account = "b", seq = 2, amount = 20),
    Posting(account = "a", seq = 3, amount = 10),
]
ledger
| over(by = t => t.account, order = t => t.seq,
       into = (t, w) => w | running | map(x => x.amount) | sum)
```

```text
[100, 50, 70, 70, 80]
```

`running` counts rows, as SQL's `ROWS BETWEEN UNBOUNDED PRECEDING AND
CURRENT ROW` does. SQL's default frame with an `ORDER BY` also takes in the
rows equal to the current one; `running` does not. A moving sum, and the
partition's `first`, `last` and `size`:

```hql
[1, 2, 3, 4]
| over(order = x => x,
       into = (x, w) => [w | rows(1, 1) | sum, w | first, w | last, w | size, w | whole | sum])
```

```text
[[3, 1, 4, 4, 10], [6, 1, 4, 4, 10], [9, 1, 4, 4, 10], [7, 1, 4, 4, 10]]
```

## The first n of each key

Keeping the two latest orders of each customer is `row_number` and a
`filter`. A record literal is `Data`, whose fields are `Option<Data>` and
not ordered, so `into` builds a declared record to compare `n` with `<=`:

```hql
type Order {
    id       : Int
    customer : String
    at       : Int
}

type Ranked {
    id : Int
    n  : Int
}

orders = [
    Order(id = 1, customer = "ada", at = 1),
    Order(id = 2, customer = "ada", at = 3),
    Order(id = 3, customer = "bob", at = 2),
    Order(id = 4, customer = "ada", at = 2),
    Order(id = 5, customer = "bob", at = 1),
    Order(id = 6, customer = "ada", at = 4),
]
orders
| over(by = o => o.customer, order = (a, b) => compare(b.at, a.at),
       into = (o, w) => Ranked(id = o.id, n = w | row_number))
| filter(x => x.n <= 2)
| map(x => x.id)
```

```text
[2, 3, 5, 6]
```

## What `over` refuses

A window is a view of its partition while `into` runs. `into` may not give
one back, in any type: the checker refuses `into = (x, w) => [w]`. And
`over` takes a `Seq`: a partition of a stream is never complete, since a
stream may not end, so `over` of a `Stream` is refused like `group` of one.
Inside `collect(n)`, `over` ranks each batch, and a row in one batch is not
compared with a row in another.
