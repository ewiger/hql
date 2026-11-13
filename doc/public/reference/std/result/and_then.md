<!-- hql reference: std/result.and_then -->
# `and_then`

In [`std/result`](index.md).

```hql
fn and_then<U>(self, by: T -> Result<U, E>) : Result<U, E>
```

`by` applied to the value of a success, its own failure kept; a failure
is kept as it is: `ok(4) | and_then(n => ok(n * 2))` is `ok(8)`.
<!-- hql reference: end -->
