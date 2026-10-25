<!-- hql reference: std/result.err -->
# `err`

In [`std/result`](index.md).

```hql
fn err(self) : Option<E>
```

The error of `self` when a failure, and `none` when a success:
`ok(4) | err` is `none`.
<!-- hql reference: end -->
