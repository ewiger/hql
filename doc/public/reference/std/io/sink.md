<!-- hql reference: std/io.sink -->
# `sink`

In [`std/io`](index.md).

```hql
effectful fn sink<T, F : Format>(values: Iterable<T>, endpoint: Writable, format: Type<F>) : Unit
```

Writes each of `values` to `endpoint` in `format`, one document per
value, and gives `Unit`: `[{a: 1}, {a: 2}] | sink(stdout, Json)` writes
`{"a":1}` and `{"a":2}` on two lines. Written to a `Channel<T>`, each
value is appended and no format is taken. Effectful: it runs once,
where it is written.
<!-- hql reference: end -->
