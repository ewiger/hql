<!-- hql reference: std/collections.contains -->
# `contains`

In [`std/collections`](index.md).

```hql
reduction fn contains<T>(self: Collection<T>, value: T, /) : Bool
```

`true` when `value` occurs in `self`, and `false` otherwise:
`[1, 2, 3] | contains(2)` is `true`.
<!-- hql reference: end -->
