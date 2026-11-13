<!-- hql reference: std/core.Seq -->
# `Seq`

In [`std/core`](../index.md).

```hql
abstract type Seq<T> : {Collection<T>, Stream<T>}
```

A finite stream: a collection in which every occurrence has exactly one
position in `0 .. size(c) - 1`. Positional order does not require
`T : Orderable`.
<!-- hql reference: end -->
