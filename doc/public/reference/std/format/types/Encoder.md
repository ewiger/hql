<!-- hql reference: std/format.Encoder -->
# `Encoder`

In [`std/format`](../index.md).

```hql
trait Encoder<T>
```

The contract that values of `T` are written as records of a format:
`format(x, F)` takes the types `F`'s encoders cover. A format writes one
type in several ways, so a format has an `Encoder` at every type it
writes, and a renderer chosen for a type in a format is one of them.
<!-- hql reference: end -->
