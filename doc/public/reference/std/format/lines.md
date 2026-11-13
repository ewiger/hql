<!-- hql reference: std/format.lines -->
# `lines`

In [`std/format`](index.md).

```hql
fn lines<T>(x: Collection<T>) : String
fn lines<T>(x: Stream<T>) : Stream<String>
fn lines<T>(x: T) : String
```

`x` encoded as `Lines`: `x | lines` is `format(x, Lines)`.
<!-- hql reference: end -->
