<!-- hql reference: std/string.trim_prefix -->
# `trim_prefix`

In [`std/string`](index.md).

```hql
fn trim_prefix(self, part: String) : String
```

`self` without `part` at its start, or `self` as it is when it does not
begin with it: `"x" | trim_prefix("y")` is `"x"`.
<!-- hql reference: end -->
