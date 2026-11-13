<!-- hql reference: std/string.index_of -->
# `index_of`

In [`std/string`](index.md).

```hql
fn index_of(self, part: String) : Option<Int>
```

The position of the first occurrence of `part` in `self`, counting
characters from 0, or `none`: `"héllo" | index_of("l")` is `2`.
<!-- hql reference: end -->
