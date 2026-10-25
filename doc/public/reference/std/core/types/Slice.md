<!-- hql reference: std/core.Slice -->
# `Slice`

In [`std/core`](../index.md).

```hql
trait Slice
```

A part: `xs[a:b]` over a type implementing `Slice` is
`xs.slice(Range(start = a, end = b))`, a value of the type itself.

## Members

```hql
fn slice(self, range: Range) : Self
```
<!-- hql reference: end -->
