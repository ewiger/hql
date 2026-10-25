<!-- hql reference: std/option.unwrap_or_else -->
# `unwrap_or_else`

In [`std/option`](index.md).

```hql
fn unwrap_or_else(self, default: () -> T) : T
```

The value of `self` when present, and what `default` returns otherwise;
`default` is called only when `self` is `none`:
`[1, 2][5] | unwrap_or_else(() => 7)` is `7`.
<!-- hql reference: end -->
