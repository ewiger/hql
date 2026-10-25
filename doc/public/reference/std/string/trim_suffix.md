<!-- hql reference: std/string.trim_suffix -->
# `trim_suffix`

In [`std/string`](index.md).

```hql
fn trim_suffix(self, part: String) : String
```

`self` without `part` at its end, or `self` as it is when it does not
end with it: `"x.hql" | trim_suffix(".hql")` is `"x"`.
<!-- hql reference: end -->
