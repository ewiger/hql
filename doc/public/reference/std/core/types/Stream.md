<!-- hql reference: std/core.Stream -->
# `Stream`

In [`std/core`](../index.md).

```hql
abstract type Stream<T> : Iterable<T>
```

An ordered, possibly unending run of values `x0, x1, x2, ...`. Size,
membership and global order are undefined on a stream: each requires a
last value.
<!-- hql reference: end -->
