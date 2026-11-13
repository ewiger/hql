<!-- hql reference: std/format.text -->
# `text`

In [`std/format`](index.md).

```hql
fn text<T>(x: Collection<T>) : String
fn text<T>(x: Stream<T>) : Stream<String>
fn text<T>(x: T) : String
```

`x` encoded as `Text`: `x | text` is `format(x, Text)`.
<!-- hql reference: end -->
