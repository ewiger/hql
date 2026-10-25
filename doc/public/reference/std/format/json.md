<!-- hql reference: std/format.json -->
# `json`

In [`std/format`](index.md).

```hql
fn json<T>(x: Collection<T>) : String
fn json<T>(x: Stream<T>) : Stream<String>
fn json<T>(x: T) : String
```

`x` encoded as `Json`: `x | json` is `format(x, Json)`.
<!-- hql reference: end -->
