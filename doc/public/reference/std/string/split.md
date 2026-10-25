<!-- hql reference: std/string.split -->
# `split`

In [`std/string`](index.md).

```hql
fn split(self, separator: String, limit: Int = 0) : List<String>
```

The pieces of `self` between occurrences of `separator`, in order:
`"a,b,,c" | split(",")` is `["a", "b", "", "c"]`. A `limit` of 1 or more
keeps at most that many pieces, the last holding the rest of the text:
`"a,b,c" | split(",", 2)` is `["a", "b,c"]`; below 1 there is no limit.
<!-- hql reference: end -->
