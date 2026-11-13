<!-- hql reference: std/result.unwrap_or -->
# `unwrap_or`

In [`std/result`](index.md).

```hql
fn unwrap_or(self, default: T) : T
```

The value of `self` when a success, and `default` when a failure:
`err("x") | unwrap_or(0)` is `0`.
<!-- hql reference: end -->
