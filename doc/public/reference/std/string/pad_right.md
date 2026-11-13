<!-- hql reference: std/string.pad_right -->
# `pad_right`

In [`std/string`](index.md).

```hql
fn pad_right(self, width: Int, fill: String) : String
```

`self` followed by `fill`, repeated, until it is `width` characters long;
a longer `self` as it is: `"7" | pad_right(3, "0")` is `"700"`.
<!-- hql reference: end -->
