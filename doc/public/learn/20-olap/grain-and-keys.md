# Grain and keys

Before a row can be a duplicate, it has to be clear what one row stands for.
That is the table's **grain**: the thing in the business that one row
describes. The shop's silver table has one row per event; its gold table
has one row per customer. A row is a duplicate when the table already holds
a row for the same thing, and "the same thing" is decided by the grain, not
by the row's contents.

The **key** is the field, or fields, that name the thing at the table's
grain. Two kinds of key are told apart:

- a **primary key** names a row and is given by whoever produced it: the
  event's `id`, which the checkout assigned. It is unique by construction
  and means nothing to the business.
- a **business key** names the thing in the business's own terms: the
  customer's name, the order's number on the invoice, the pair of a customer
  and a day. It is what two systems that never shared an `id` agree on.

The silver table's key is the primary key `id`. The gold table's key is the
business key `customer`. A table at the grain of a customer-day has the
composite business key of the two.

## The replay

`silver_replayed` is the shop's six valid events as a replay of the subject
delivered them. Reading it as `Event`s, the rows, the distinct rows, and the
distinct ids are three different counts:

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
[rows | size, rows | distinct | size, rows | map(r => r.id) | distinct | size]
```

```text
[9, 7, 6]
```

Nine rows arrived. Seven are distinct as values: two events were delivered
twice, identical to the byte. Six ids are distinct: one event was delivered
twice with different contents, a correction. At the grain of an event, the
table should hold six rows, and which six is the question of the
[next page](deduplication.md).

The ids that occur more than once, with how often and with which amounts:

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
rows : Seq<Event> = source("silver_replayed")
rows
| group(by = r => r.id)
| filter(g => (g.items | size) > 1)
| map(g => {id: g.key, copies: g.items | size, amounts: g.items | map(r => r.amount) | distinct})
```

```text
{{amounts: [42.00], copies: 2, id: 2}, {amounts: [7.99], copies: 2, id: 4}, {amounts: [15.25, 18.00], copies: 2, id: 6}}
```

Order 6 is the one whose copies disagree: placed for 15.25, then corrected
to 18.00 five minutes later.

## Another grain

The same rows at another grain are a different table with a different key.
One row per customer per day, keyed by both:

<!-- run: with Event --vault examples/pipelines/shop -->
```hql
rows : Seq<Event> = source("silver_replayed")
rows
| group(by = r => {customer: r.customer, day: r.at.slice(0, 10)})
| map(g => {customer: g.key.customer, day: g.key.day, events: g.items | size})
```

```text
{{customer: "ada", day: "2026-11-01", events: 1}, {customer: "grace", day: "2026-11-01", events: 3}, {customer: "linus", day: "2026-11-01", events: 2}, {customer: "ada", day: "2026-11-02", events: 3}}
```

The counts are wrong, because they were taken over the replay and not over
the deduplicated events: grace has two events on the first day, not three.
A gold table is only as right as the silver table it is built from, which is
why deduplication is silver's job and is done once, at the grain of a fact,
before any report is built at any other grain.

## What a key is to the checker

A declared record has no field marked as its key, and the configuration of
a table has none yet either. The key is stated where it is used: in the
`by` of a `group`, in the `left` and `right` of a `join`, in the `key` of a
`to_map`, which refuses two values under one key as an evaluation error, and,
as design, in the `key` of a write mode on the [merge page](merge.md).
