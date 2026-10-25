<!-- hql reference: std/result.map_err -->
# `map_err`

In [`std/result`](index.md).

```hql
fn map_err<U>(self, by: E -> U) : Result<T, U>
```

A failure with `by` applied to its error; a success is kept as it is:
`err("x") | map_err(e => e.upper())` is `err("X")`.
<!-- hql reference: end -->
