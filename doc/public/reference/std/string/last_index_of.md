<!-- hql reference: std/string.last_index_of -->
# `last_index_of`

In [`std/string`](index.md).

```hql
fn last_index_of(self, part: String) : Option<Int>
```

The position of the last occurrence of `part` in `self`, counting
characters from 0, or `none`: `"héllo" | last_index_of("l")` is `3`.
<!-- hql reference: end -->
