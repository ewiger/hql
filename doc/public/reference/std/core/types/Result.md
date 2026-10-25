<!-- hql reference: std/core.Result -->
# `Result`

In [`std/core`](../index.md).

```hql
type Result<T, E>
```

A computation that may have failed: `ok(value)` with a `T`, or
`err(error)` with an `E`. The ways out are the `result` module's, and
mirror `Option`'s; there is no unwrap.
<!-- hql reference: end -->
