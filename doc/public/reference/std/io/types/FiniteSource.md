<!-- hql reference: std/io.FiniteSource -->
# `FiniteSource`

In [`std/io`](../index.md).

```hql
abstract type FiniteSource<T> : {DataSource, Seq<T>}
```

A source of every document, in order, that ends: read from a
`Replayable` endpoint.
<!-- hql reference: end -->
