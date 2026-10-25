<!-- hql reference: std/result.ok -->
# `ok`

In [`std/result`](index.md).

```hql
fn ok(self) : Option<T>
```

The value of `self` when a success, and `none` when a failure:
`err("x") | ok` is `none`.
<!-- hql reference: end -->
