<!-- hql reference: std/io.target -->
# `target`

In [`std/io`](index.md).

```hql
effectful fn target<T, F : Format>(value: T, endpoint: Writable, format: Type<F>) : Unit
effectful fn target<T, F : Format>(value: T, id: String, format: Type<F>) : Unit
```

Writes `value` as one document to `endpoint` in `format`, or to the
target configured under `id`, and gives `Unit`. A `Seq` is one
document, in `Json` an array; a `Stream` that is not a `Seq` is a type error:
write it with `sink`, or bound it with `take(n)` first. Written to a
`Channel<T>`, `value` is appended and no format is taken. Effectful: it
runs once, where it is written.
<!-- hql reference: end -->
