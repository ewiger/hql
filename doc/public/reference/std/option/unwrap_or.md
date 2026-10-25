<!-- hql reference: std/option.unwrap_or -->
# `unwrap_or`

In [`std/option`](index.md).

```hql
fn unwrap_or(self, default: T) : T
```

The value of `self` when present, and `default` otherwise:
`[1, 2][5] | unwrap_or(0)` is `0`.
<!-- hql reference: end -->
