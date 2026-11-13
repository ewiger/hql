<!-- hql reference: std/string.to_int -->
# `to_int`

In [`std/string`](index.md).

```hql
fn to_int(self) : Option<Int>
```

The integer `self` writes, surrounding whitespace aside, or `none` when
it writes none: `"-42" | to_int` is `-42`, and `"1.5" | to_int` is
`none`.
<!-- hql reference: end -->
