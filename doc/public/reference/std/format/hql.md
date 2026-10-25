<!-- hql reference: std/format.hql -->
# `hql`

In [`std/format`](index.md).

```hql
fn hql<T>(x: Collection<T>) : String
fn hql<T>(x: Stream<T>) : Stream<String>
fn hql<T>(x: T) : String
```

`x` encoded as `Hql`: `x | hql` is `format(x, Hql)`.
<!-- hql reference: end -->
