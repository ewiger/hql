<!-- hql reference: std/io.source -->
# `source`

In [`std/io`](index.md).

```hql
effectful fn source<F : Format, T>(endpoint: Readable, format: Type<F>) : Iterable<T>
effectful fn source<F : Format, T>(id: String, format: Type<F>) : Iterable<T>
```

Every document read from `endpoint` in `format`, decoded to `T`: a `Seq`
from a `Replayable` endpoint, a `Stream` from a live one. `T` is the
type the format decodes to, or the element type the read is expected to
give: `s : Seq<Sighting> = source(file("birds.json"))` decodes each
document as a `Sighting`. The format may be omitted when the endpoint is
`file` of a literal path whose extension implies one. `source(id)` reads
the source configured under the literal `id` in hql.toml, as the type
its kind states, and takes no format; `source(c)` of a `Channel<T>` is a
`Seq<T>` of what it holds now. Effectful: it runs once, where it is
written.
<!-- hql reference: end -->
