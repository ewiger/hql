<!-- hql reference: std/result.or_else -->
# `or_else`

In [`std/result`](index.md).

```hql
fn or_else<U>(self, by: E -> Result<T, U>) : Result<T, U>
```

`by` applied to the error of a failure, giving another result; a success
is kept as it is: `err("x") | or_else(e => ok(e.size()))` is `ok(1)`.
<!-- hql reference: end -->
