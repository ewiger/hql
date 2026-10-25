<!-- hql reference: std/collections.count -->
# `count`

In [`std/collections`](index.md).

```hql
reduction fn count<T>(self: Collection<T>) : Int
```

The number of element occurrences in `self`, as `size` gives it: `count`
is the kept alias of `size`. `[1, 2, 2, 3] | count` is `4`.

```hql
reduction fn count<T>(self: Collection<T>, value: T, /) : Int
```

The number of occurrences in `self` equal to `value`:
`[1, 2, 2, 3] | count(2)` is `2`. `value` is passed by position only.
<!-- hql reference: end -->
