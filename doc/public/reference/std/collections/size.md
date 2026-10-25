<!-- hql reference: std/collections.size -->
# `size`

In [`std/collections`](index.md).

```hql
reduction fn size<T>(self: Collection<T>) : Int
```

The number of element occurrences in `self`, an `Int`:
`[1, 2, 2, 3] | size` is `4`, and `Set([1, 2, 2]) | size` is `2`.
<!-- hql reference: end -->
