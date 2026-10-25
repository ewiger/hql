<!-- hql reference: std/collections.emit -->
# `emit`

In [`std/collections`](index.md).

```hql
elementwise fn emit<T>(self: Collection<Seq<T>>) : List<T>
elementwise fn emit<T>(self: Stream<Seq<T>>) : Stream<T>
```

Every member of every sequence, in order, the inverse of `collect`:
`[[1, 2], [3]] | emit` is `[1, 2, 3]`.
<!-- hql reference: end -->
