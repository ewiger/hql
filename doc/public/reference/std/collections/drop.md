<!-- hql reference: std/collections.drop -->
# `drop`

In [`std/collections`](index.md).

```hql
fn drop<S : Iterable<T>, T>(self: S, n: Int) : S
```

The elements after the first `n`, in the input's order and type:
`[5, 3, 4] | drop(1)` is `[3, 4]`. The checker refuses an input without
a stable order, such as a `Set`.
<!-- hql reference: end -->
