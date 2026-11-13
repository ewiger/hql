<!-- hql reference: std/collections.reduce -->
# `reduce`

In [`std/collections`](index.md).

```hql
fn reduce<T, R>(self: Collection<T>, initial: R, by: (R, T) -> R) : R
fn reduce<T>(self: Collection<T>, by: (T, T) -> T) : Option<T>
```

The elements folded from the left by `by`, from `initial`, or without
it from the first element, the result then `none` over `[]`:
`[1, 2, 3] | reduce(0, (a, x) => a + x)` is `6`. The checker rejects a
`Set`, which has no positions to fold in.
<!-- hql reference: end -->
