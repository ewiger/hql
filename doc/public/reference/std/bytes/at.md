<!-- hql reference: std/bytes.at -->
# `at`

In [`std/bytes`](index.md).

```hql
fn at(self, position: Int) : Option<Int>
```

The octet of `self` at `position`, as an `Int` in `0..255`, or `none`
outside it; a negative position counts from the end:
`Bytes("hi").at(0)` and `Bytes("hi")[0]` are `104`.
<!-- hql reference: end -->
