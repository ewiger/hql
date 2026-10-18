# Examples

The example corpus is where HQL's features are designed: an example is written
first, and the implementation is held to it. Each query file declares its
expected type and result in a header, and the test suite runs every one.

| Corpus | What it shows |
| --- | --- |
| [birds](https://github.com/ewiger/hql/tree/main/examples/birds) | Linked cards with runnable collection queries: repeated sightings, distinct species, keyed counts, ordered routes and sorted keys — plus intentional errors for duplicate keys and non-orderable sorted-map keys |
| [mapreduce](https://github.com/ewiger/hql/tree/main/examples/mapreduce) | Six book cards to filter, group and reduce. Some queries run today; others are written ahead of `group`, `reduce` and `collect` |

## Reading a query header

```hql
// status: valid-now
// feature: Bird cards filtered and sorted by name
// implementation: implemented
// vault: birds/wiki
// expected-type: Seq<String>
// expected: [Barn owl, Common raven, European robin]
```

A header may instead say `status: proposed` with `implementation: pending` and
a `proposal:` naming the record that specifies it. Such a case is a design
input, pinned before it runs, and the suite asserts that it does *not* check
yet — the day it does, it has to be promoted.

## Running one

From a clone, after `cargo build --locked`:

```sh
target/debug/hql --vault examples/birds/wiki run examples/birds/queries/catalog.hql
```
