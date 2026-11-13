<!-- hql reference: std/io.sink -->
# `sink`

In [`std/io`](index.md).

```hql
effectful fn sink<T, F : Format>(values: Iterable<T>, endpoint: Writable, format: Type<F>) : Step
effectful fn sink<T, F : Format>(values: Iterable<T>, id: String, format: Type<F>) : Step
```

Writes each of `values` to `endpoint` in `format`, one document per
value, and gives `Step`: `[{a: 1}, {a: 2}] | sink(stdout, Json)` writes
`{"a":1}` and `{"a":2}` on two lines. Written to a `Channel<T>`, each
value is appended and no format is taken. `sink(id)` writes to the
target configured under `id` as each value arrives: a `kafka` topic or
a `nats` or `nats_jetstream` subject takes one message per value, a `postgres` or a `spark`
table one row per value of a declared record. Effectful: it runs once, where it is written.
<!-- hql reference: end -->
