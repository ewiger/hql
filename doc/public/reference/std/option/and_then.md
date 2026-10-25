<!-- hql reference: std/option.and_then -->
# `and_then`

In [`std/option`](index.md).

```hql
fn and_then<U>(self, by: T -> Option<U>) : Option<U>
```

`by` applied to the value of `self` when present, its own `none` kept,
and `none` otherwise: `[4][0] | and_then(x => [10][x])` is `none`.
<!-- hql reference: end -->
