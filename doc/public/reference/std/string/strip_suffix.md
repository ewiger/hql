<!-- hql reference: std/string.strip_suffix -->
# `strip_suffix`

In [`std/string`](index.md).

```hql
fn strip_suffix(self, part: String) : Option<String>
```

`self` without `part` at its end, or `none` when it does not end with
it: `"x.hql" | strip_suffix(".hql")` is `"x"`.
<!-- hql reference: end -->
