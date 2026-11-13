<!-- hql reference: std/string.occurrences -->
# `occurrences`

In [`std/string`](index.md).

```hql
fn occurrences(self, part: String) : Int
```

How many non-overlapping times `part` occurs in `self`; 0 for an empty
`part`: `"aaaa" | occurrences("aa")` is `2`.
<!-- hql reference: end -->
