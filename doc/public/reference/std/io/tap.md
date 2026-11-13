<!-- hql reference: std/io.tap -->
# `tap`

In [`std/io`](index.md).

```hql
effectful fn tap<F : Format, T>(endpoint: Readable, format: Type<F>) : Stream<T>
effectful fn tap<F : Format, T>(id: String, format: Type<F>) : Stream<T>
```

The documents read from `endpoint` in `format`, one held at a time, as a
`Stream<T>`. A format that is one whole document, such as `Text`, is
a type error. `tap(id)` gives the elements of the finite source configured
under `id` as a `Stream`; `tap(c)` of a `Channel<T>` gives each value as
it is written; `tap(file(path).end, F)` follows what is appended.
Effectful: it runs once, where it is written.
<!-- hql reference: end -->
