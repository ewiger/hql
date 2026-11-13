<!-- hql reference: std/string.bytes -->
# `bytes`

In [`std/string`](index.md).

```hql
fn bytes(self) : List<Int>
```

The UTF-8 encoding of `self`, one `Int` per byte: `"é" | bytes` is
`[195, 169]`.
<!-- hql reference: end -->
