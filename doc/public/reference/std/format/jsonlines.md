<!-- hql reference: std/format.jsonlines -->
# `jsonlines`

In [`std/format`](index.md).

```hql
fn jsonlines<T>(x: Collection<T>) : String
fn jsonlines<T>(x: Stream<T>) : Stream<String>
fn jsonlines<T>(x: T) : String
```

`x` encoded as `JsonLines`: `x | jsonlines` is `format(x, JsonLines)`.
<!-- hql reference: end -->
