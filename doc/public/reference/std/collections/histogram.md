<!-- hql reference: std/collections.histogram -->
# `histogram`

In [`std/collections`](index.md).

```hql
reduction fn histogram(self: Collection<Int>, width: Int) : SortedMap<Int, Int>
reduction fn histogram(self: Collection<Float>, width: Float) : SortedMap<Float, Int>
```

The number of occurrences in each bucket of `width`, keyed by the
bucket's lower bound, the greatest multiple of `width` not above the
value, in key order: `[1, 5, 12, -3] | histogram(5)` is
`SortedMap {-5: 1, 0: 1, 5: 1, 10: 1}`. A `width` of zero or less is an
evaluation error.
<!-- hql reference: end -->
