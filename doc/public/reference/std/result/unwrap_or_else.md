<!-- hql reference: std/result.unwrap_or_else -->
# `unwrap_or_else`

In [`std/result`](index.md).

```hql
fn unwrap_or_else(self, default: E -> T) : T
```

The value of `self` when a success, and `default` applied to the error
when a failure: `err("ab") | unwrap_or_else(e => e.size())` is `2`.
<!-- hql reference: end -->
