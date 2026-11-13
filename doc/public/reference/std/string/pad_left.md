<!-- hql reference: std/string.pad_left -->
# `pad_left`

In [`std/string`](index.md).

```hql
fn pad_left(self, width: Int, fill: String) : String
```

`self` preceded by `fill`, repeated, until it is `width` characters long;
a longer `self` as it is: `"7" | pad_left(3, "0")` is `"007"`.
<!-- hql reference: end -->
