<!-- hql reference: std/format.mermaid -->
# `mermaid`

In [`std/format`](index.md).

```hql
fn mermaid<T>(x: Collection<T>) : String
fn mermaid<T>(x: Stream<T>) : Stream<String>
fn mermaid<T>(x: T) : String
```

`x` encoded as `Mermaid`: `x | mermaid` is `format(x, Mermaid)`.
<!-- hql reference: end -->
