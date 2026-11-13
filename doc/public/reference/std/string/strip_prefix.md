<!-- hql reference: std/string.strip_prefix -->
# `strip_prefix`

In [`std/string`](index.md).

```hql
fn strip_prefix(self, part: String) : Option<String>
```

`self` without `part` at its start, or `none` when it does not begin
with it: `"x.hql" | strip_prefix("x.")` is `"hql"`.
<!-- hql reference: end -->
