<!-- hql reference: std/format.markdown -->
# `markdown`

In [`std/format`](index.md).

```hql
fn markdown<T>(x: Collection<T>) : String
fn markdown<T>(x: Stream<T>) : Stream<String>
fn markdown<T>(x: T) : String
```

`x` encoded as `Markdown`: `x | markdown` is `format(x, Markdown)`.
<!-- hql reference: end -->
