<!-- hql reference: std/bytes.slice -->
# `slice`

In [`std/bytes`](index.md).

```hql
fn slice(self, start: Int, end: Int) : Bytes
fn slice(self, range: Range) : Bytes
```

The octets of `self` from `start` to `end`, clamped to its size; a
negative bound counts from the end. With one argument, a `Range`, it is
`Slice`'s: `b[0:12]` is `b.slice(Range(start = 0, end = 12))`.
<!-- hql reference: end -->
