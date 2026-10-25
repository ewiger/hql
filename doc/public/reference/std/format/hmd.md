<!-- hql reference: std/format.hmd -->
# `hmd`

In [`std/format`](index.md).

```hql
fn hmd<T>(x: Collection<T>) : String
fn hmd<T>(x: Stream<T>) : Stream<String>
fn hmd<T>(x: T) : String
```

`x` encoded as `Hmd`: `x | hmd` is `format(x, Hmd)`.
<!-- hql reference: end -->
